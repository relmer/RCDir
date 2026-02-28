// multi_threaded_lister.rs — Multi-threaded recursive directory enumeration
//
// Port of: MultiThreadedLister.h, MultiThreadedLister.cpp
//
// Producer-consumer pattern: worker threads enumerate directories in parallel,
// main thread walks the tree depth-first for in-order streaming output.

use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use windows::Win32::Storage::FileSystem::{
    FindFirstFileW, FindNextFileW, WIN32_FIND_DATAW,
};

use crate::command_line::CommandLine;
use crate::config::Config;
use crate::directory_info::{DirectoryInfo, DirectoryStatus};
use crate::drive_info::DriveInfo;
use crate::file_comparator;
use crate::file_info::{FileInfo, FindHandle, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT};
use crate::listing_totals::ListingTotals;
use crate::results_displayer::{DirectoryLevel, Displayer, ResultsDisplayer, TreeDisplayer};
use crate::tree_connector_state::TreeConnectorState;
use crate::work_queue::WorkQueue;





/// A work item is a reference to a directory node in the tree.
type WorkItem = Arc<(Mutex<DirectoryInfo>, Condvar)>;





/// Multi-threaded recursive directory lister.
///
/// Port of: CMultiThreadedLister
pub struct MultiThreadedLister {
    cmd:                    Arc<CommandLine>,
    _config:                Arc<Config>,
    work_queue:             Arc<WorkQueue<WorkItem>>,
    stop:                   Arc<AtomicBool>,
    tree_pruning_active:    Arc<AtomicBool>,
    workers:                Vec<JoinHandle<()>>,
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl MultiThreadedLister
//
//  Multi-threaded directory enumeration and worker management.
//
////////////////////////////////////////////////////////////////////////////////

impl MultiThreadedLister {
    ////////////////////////////////////////////////////////////////////////////
    //
    //  new
    //
    //  Create a new multi-threaded lister and spawn worker threads.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn new(cmd: Arc<CommandLine>, config: Arc<Config>) -> Self {
        let work_queue          = Arc::new (WorkQueue::new());
        let stop                = Arc::new (AtomicBool::new (false));
        let tree_pruning_active = Arc::new (AtomicBool::new (false));

        let num_threads = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .max(1);

        let mut workers = Vec::with_capacity(num_threads);

        for _ in 0..num_threads {
            let wq  = Arc::clone (&work_queue);
            let st  = Arc::clone (&stop);
            let tpa = Arc::clone (&tree_pruning_active);
            let c   = Arc::clone (&cmd);
            let cf  = Arc::clone (&config);

            workers.push(thread::spawn(move || {
                worker_thread_func (&wq, &st, &tpa, &c, &cf);
            }));
        }

        MultiThreadedLister { cmd, _config: config, work_queue, stop, tree_pruning_active, workers }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  process
    //
    //  Process a directory tree with multi-threaded enumeration.
    //
    //  Port of: CMultiThreadedLister::ProcessDirectoryMultiThreaded
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn process(
        &mut self,
        drive_info: &DriveInfo,
        dir_path: &Path,
        file_specs: &[OsString],
        displayer: &mut Displayer,
        totals: &mut ListingTotals,
    ) {
        let spec_strings: Vec<String> = file_specs.iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();

        let is_tree = self.cmd.tree.unwrap_or (false);
        let all_star = spec_strings.iter().all (|s| s == "*");
        self.tree_pruning_active.store (is_tree && !all_star, Ordering::Release);

        let root = DirectoryInfo::new_multi(dir_path.to_path_buf(), spec_strings);
        let root_node: WorkItem = Arc::new((Mutex::new(root), Condvar::new()));

        self.work_queue.push(Arc::clone(&root_node));

        // Consume the tree on the main thread (streaming output)
        if is_tree {
            if let Displayer::Tree (tree_displayer) = displayer {
                let mut tree_state = TreeConnectorState::new (self.cmd.tree_indent);

                self.print_directory_tree_mode (
                    &root_node,
                    drive_info,
                    tree_displayer,
                    DirectoryLevel::Initial,
                    totals,
                    &mut tree_state,
                );
            }
        } else {
            self.print_directory_tree(&root_node, drive_info, displayer, DirectoryLevel::Initial, totals);
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  stop_workers
    //
    //  Stop all worker threads and join them.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn stop_workers(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.work_queue.set_done();

        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  print_directory_tree
    //
    //  Recursive depth-first tree walk — consumes results in discovery order.
    //
    //  Port of: CMultiThreadedLister::PrintDirectoryTree
    //
    ////////////////////////////////////////////////////////////////////////////

    fn print_directory_tree(
        &self,
        node: &WorkItem,
        drive_info: &DriveInfo,
        displayer: &mut Displayer,
        level: DirectoryLevel,
        totals: &mut ListingTotals,
    ) {
        if self.stop_requested() {
            return;
        }

        // Wait for node completion
        let (status, error_msg) = wait_for_node_completion(node, &self.stop);
        if self.stop_requested() {
            return;
        }

        if status == DirectoryStatus::Error {
            if let Some(msg) = error_msg {
                let console = displayer.console_mut();
                console.color_printf(&format!(
                    "{{Error}}  Error accessing directory: {}\n", msg,
                ));
            }
            return;
        }

        // Sort, display, accumulate — all under the lock
        {
            let mut di = node.0.lock().unwrap();

            file_comparator::sort_files(&mut di.matches, &self.cmd, false);
            displayer.display_results(drive_info, &di, level);
            accumulate_totals(&di, totals);
        }

        // Collect children refs while holding the lock briefly
        let children: Vec<WorkItem> = {
            let di = node.0.lock().unwrap();
            di.children.clone()
        };

        // Recurse into children depth-first
        for child in &children {
            if self.stop_requested() {
                break;
            }
            self.print_directory_tree(child, drive_info, displayer, DirectoryLevel::Subdirectory, totals);
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  print_directory_tree_mode
    //
    //  Tree-mode depth-first walk — interleaved display with connectors.
    //
    //  Port of: CMultiThreadedLister::PrintDirectoryTreeMode
    //
    ////////////////////////////////////////////////////////////////////////////

    fn print_directory_tree_mode (
        &self,
        node: &WorkItem,
        drive_info: &DriveInfo,
        tree_displayer: &mut TreeDisplayer,
        level: DirectoryLevel,
        totals: &mut ListingTotals,
        tree_state: &mut TreeConnectorState,
    ) {
        if self.stop_requested() {
            return;
        }

        // Wait for node completion
        let (status, error_msg) = wait_for_node_completion (node, &self.stop);
        if self.stop_requested() {
            return;
        }

        if status == DirectoryStatus::Error {
            if let Some (msg) = error_msg {
                let console = tree_displayer.console_mut();
                console.color_printf (&format! (
                    "{{Error}}  Error accessing directory: {}\n", msg,
                ));
            }
            return;
        }

        // Sort with interleaved ordering (dirs and files together)
        {
            let mut di = node.0.lock().unwrap();
            file_comparator::sort_files (&mut di.matches, &self.cmd, true);
        }

        // Root directory: show drive header, path header, empty-dir message
        if level == DirectoryLevel::Initial {
            let di = node.0.lock().unwrap();
            tree_displayer.display_tree_root_header (drive_info, &di);

            if di.matches.is_empty() && di.children.is_empty() {
                tree_displayer.display_tree_empty_root_message (&di);
                return;
            }
        }

        // Compute per-directory display state
        {
            let di = node.0.lock().unwrap();
            tree_displayer.begin_directory (&di);
        }

        // Display entries interleaved with directory recursion
        self.display_tree_entries (node, drive_info, tree_displayer, totals, tree_state);

        // Flush trailing output
        let _ = tree_displayer.console_mut().flush();

        // Accumulate totals
        {
            let di = node.0.lock().unwrap();
            accumulate_totals (&di, totals);
        }

        // Root directory: show summary + separator
        if level == DirectoryLevel::Initial {
            tree_displayer.display_tree_root_summary();
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  display_tree_entries
    //
    //  Display each visible entry interleaved with directory recursion.
    //  Builds a child lookup map from lowercase filename to child WorkItem,
    //  then iterates entries determining last-entry status for connectors.
    //
    //  Port of: CMultiThreadedLister::DisplayTreeEntries
    //
    ////////////////////////////////////////////////////////////////////////////

    fn display_tree_entries (
        &self,
        node: &WorkItem,
        drive_info: &DriveInfo,
        tree_displayer: &mut TreeDisplayer,
        totals: &mut ListingTotals,
        tree_state: &mut TreeConnectorState,
    ) {
        // Build lookup from lowercase filename → child WorkItem
        let (child_map, entries) = {
            let di = node.0.lock().unwrap();

            let mut child_map: HashMap<String, WorkItem> = HashMap::new();
            for child in &di.children {
                let child_di = child.0.lock().unwrap();
                let child_name = child_di.dir_path
                    .file_name()
                    .map (|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                child_map.insert (child_name, Arc::clone (child));
            }

            // Clone the matches for iteration outside the lock
            let entries: Vec<FileInfo> = di.matches.clone();

            (child_map, entries)
        };

        let pruning = self.pruning_active();

        for (i, entry) in entries.iter().enumerate() {
            if self.stop_requested() {
                return;
            }

            let is_dir = (entry.file_attributes & FILE_ATTRIBUTE_DIRECTORY) != 0;

            // Tree pruning: if this directory has no descendant matches, skip it
            if pruning && is_dir {
                let lower_name = entry.file_name.to_string_lossy().to_lowercase();
                if let Some (child_node) = child_map.get (&lower_name) {
                    let visible = wait_for_tree_visibility (child_node, &self.stop);
                    if !visible {
                        // Pruned — decrement subdirectory count so totals are correct
                        let mut di = node.0.lock().unwrap();
                        if di.subdirectory_count > 0 {
                            di.subdirectory_count -= 1;
                        }
                        continue;
                    }
                }
            }

            // Determine if this is the last visible entry (look-ahead)
            let is_last = self.is_last_visible_entry (&entries, i, &child_map);

            tree_displayer.display_single_entry (entry, tree_state, is_last, i);

            // If entry is a directory, find its child node and recurse
            if is_dir {
                let lower_name = entry.file_name.to_string_lossy().to_lowercase();
                if let Some (child_node) = child_map.get (&lower_name) {
                    self.recurse_into_child_directory (
                        child_node,
                        entry,
                        is_last,
                        drive_info,
                        tree_displayer,
                        totals,
                        tree_state,
                    );
                }
            }
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  recurse_into_child_directory
    //
    //  Recurse into a child directory: check depth limit, save/restore
    //  display state, push/pop tree connector state.
    //
    //  Port of: CMultiThreadedLister::RecurseIntoChildDirectory
    //
    ////////////////////////////////////////////////////////////////////////////

    #[allow(clippy::too_many_arguments)]
    fn recurse_into_child_directory (
        &self,
        child_node: &WorkItem,
        parent_entry: &FileInfo,
        is_last: bool,
        drive_info: &DriveInfo,
        tree_displayer: &mut TreeDisplayer,
        totals: &mut ListingTotals,
        tree_state: &mut TreeConnectorState,
    ) {
        // Check depth limiting
        let depth_limited = self.cmd.max_depth > 0
            && (tree_state.depth() + 1) >= self.cmd.max_depth as usize;

        // Check reparse point (junctions/symlinks)
        let is_reparse = (parent_entry.file_attributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0;

        if depth_limited || is_reparse {
            return;
        }

        // Flush before recursing so user sees output immediately
        let _ = tree_displayer.console_mut().flush();

        // Save parent's per-directory display state
        let saved_state = tree_displayer.save_directory_state();

        // Push tree connector state: !is_last means parent has more siblings
        tree_state.push (!is_last);

        self.print_directory_tree_mode (
            child_node,
            drive_info,
            tree_displayer,
            DirectoryLevel::Subdirectory,
            totals,
            tree_state,
        );

        tree_state.pop();

        // Restore parent's per-directory display state
        tree_displayer.restore_directory_state (saved_state);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_last_visible_entry
    //
    //  Look ahead from the current entry to determine if it is the last
    //  visible entry.  Without tree pruning active, every entry after
    //  the current one is visible.
    //
    //  Port of: CMultiThreadedLister::IsLastVisibleEntry
    //
    ////////////////////////////////////////////////////////////////////////////

    fn is_last_visible_entry (
        &self,
        entries: &[FileInfo],
        current_idx: usize,
        child_map: &HashMap<String, WorkItem>,
    ) -> bool {
        let pruning = self.pruning_active();

        // Look ahead from the current position
        for next_entry in &entries[(current_idx + 1)..] {
            let next_is_dir = (next_entry.file_attributes & FILE_ATTRIBUTE_DIRECTORY) != 0;

            if !next_is_dir {
                // A file entry is always visible
                return false;
            }

            // Directory entry: if pruning is not active, it's always visible
            if !pruning {
                return false;
            }

            // Pruning active: check if this directory will be visible
            let lower_name = next_entry.file_name.to_string_lossy().to_lowercase();
            if let Some (child_node) = child_map.get (&lower_name) {
                if wait_for_tree_visibility (child_node, &self.stop) {
                    return false;
                }
                // Not visible — keep scanning
            } else {
                // No matching child node; directory is still visible
                return false;
            }
        }

        // No more visible entries found
        true
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  stop_requested
    //
    //  Check if a stop has been requested.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn stop_requested(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  pruning_active
    //
    //  Check if tree pruning is active.
    //
    ////////////////////////////////////////////////////////////////////////////

    fn pruning_active(&self) -> bool {
        self.tree_pruning_active.load (Ordering::Acquire)
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl Drop for MultiThreadedLister
//
//  Stop workers on drop.
//
////////////////////////////////////////////////////////////////////////////////

impl Drop for MultiThreadedLister {
    fn drop(&mut self) {
        self.stop_workers();
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  worker_thread_func
//
//  Worker thread function — processes items from the work queue.
//
//  Port of: CMultiThreadedLister::WorkerThreadFunc
//
////////////////////////////////////////////////////////////////////////////////

fn worker_thread_func(
    work_queue: &WorkQueue<WorkItem>,
    stop: &AtomicBool,
    tree_pruning_active: &AtomicBool,
    cmd: &CommandLine,
    _config: &Config,
) {
    while !stop.load(Ordering::Acquire) {
        let item = match work_queue.pop() {
            Some(item) => item,
            None => break, // Queue is done
        };

        enumerate_directory_node (&item, work_queue, stop, tree_pruning_active, cmd);
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  enumerate_directory_node
//
//  Enumerate a single directory node (producer function).
//
//  Port of: CMultiThreadedLister::EnumerateDirectoryNode
//
////////////////////////////////////////////////////////////////////////////////

fn enumerate_directory_node(
    node: &WorkItem,
    work_queue: &WorkQueue<WorkItem>,
    stop: &AtomicBool,
    tree_pruning_active: &AtomicBool,
    cmd: &CommandLine,
) {
    // Set InProgress
    {
        let mut di = node.0.lock().unwrap();
        di.status = DirectoryStatus::InProgress;
    }

    let result = perform_enumeration (node, work_queue, stop, tree_pruning_active, cmd);

    let pruning = tree_pruning_active.load (Ordering::Acquire);

    // Set Done or Error, and gather pruning info (under lock)
    let should_propagate = {
        let mut di = node.0.lock().unwrap();
        match result {
            Ok(()) => di.status = DirectoryStatus::Done,
            Err(msg) => {
                di.status = DirectoryStatus::Error;
                di.error = Some(msg);
            }
        }

        // If pruning is active and this node has matching files,
        // mark this node and extract parent ref for propagation.
        if pruning && di.file_count > 0 {
            di.descendant_match_found = true;
            di.parent.as_ref().map (|w| w.clone())
        } else {
            None
        }
    };

    // Propagate match flag up the ancestor chain (outside the lock
    // to avoid deadlock with signal_subtree_complete which locks
    // parent then child).
    if let Some (ref parent_weak) = should_propagate {
        propagate_descendant_match (parent_weak);
    }

    // If pruning is active, check if this is a leaf (no children) and
    // signal subtree complete upward.
    if pruning {
        let is_leaf = {
            let di = node.0.lock().unwrap();
            di.children.is_empty()
        };

        if is_leaf {
            signal_subtree_complete (node);
        }
    }

    // Notify consumer
    node.1.notify_one();
}





////////////////////////////////////////////////////////////////////////////////
//
//  perform_enumeration
//
//  Perform the actual enumeration: matching files + subdirectories.
//
//  Port of: CMultiThreadedLister::PerformEnumeration
//
////////////////////////////////////////////////////////////////////////////////

fn perform_enumeration(
    node: &WorkItem,
    work_queue: &WorkQueue<WorkItem>,
    stop: &AtomicBool,
    tree_pruning_active: &AtomicBool,
    cmd: &CommandLine,
) -> Result<(), String> {
    enumerate_matching_files (node, stop, cmd)?;

    if cmd.recurse || cmd.tree.unwrap_or (false) {
        enumerate_subdirectories (node, work_queue, stop, tree_pruning_active, cmd)?;
    }

    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  enumerate_matching_files
//
//  Enumerate files matching the file specs, with deduplication across specs.
//
//  Port of: CMultiThreadedLister::EnumerateMatchingFiles
//
////////////////////////////////////////////////////////////////////////////////

fn enumerate_matching_files(
    node: &WorkItem,
    stop: &AtomicBool,
    cmd: &CommandLine,
) -> Result<(), String> {
    // Read dir_path and file_specs while holding the lock briefly
    let (dir_path, file_specs) = {
        let di = node.0.lock().unwrap();
        (di.dir_path.clone(), di.file_specs.clone())
    };

    let mut seen: HashSet<String> = HashSet::new();

    for spec in &file_specs {
        if stop.load(Ordering::Acquire) { break; }

        let mut search_path = dir_path.clone();
        search_path.push(spec);
        let search_wide: Vec<u16> = search_path.as_os_str().encode_wide().chain(Some(0)).collect();

        let mut wfd = WIN32_FIND_DATAW::default();
        let handle = unsafe { FindFirstFileW(windows::core::PCWSTR(search_wide.as_ptr()), &mut wfd) };
        let handle = match handle {
            Ok(h) if !h.is_invalid() => h,
            _ => continue,
        };
        let _find_handle = FindHandle(handle);

        loop {
            if stop.load(Ordering::Acquire) { break; }

            if !is_dots(&wfd.cFileName) {
                // Dedup across multiple file specs
                let name_len = wfd.cFileName.iter().position(|&c| c == 0).unwrap_or(0);
                let lower_name = OsString::from_wide(&wfd.cFileName[..name_len])
                    .to_string_lossy().to_lowercase();

                if !seen.contains(&lower_name) {
                    seen.insert(lower_name);

                    let attrs = wfd.dwFileAttributes;
                    let required_ok = (attrs & cmd.attrs_required) == cmd.attrs_required;
                    let excluded_ok = (attrs & cmd.attrs_excluded) == 0;

                    if required_ok && excluded_ok {
                        let file_entry = FileInfo::from_find_data(&wfd);
                        let mut di = node.0.lock().unwrap();
                        add_match_to_list(&wfd, file_entry, &mut di, cmd);
                    }
                }
            }

            let success = unsafe { FindNextFileW(handle, &mut wfd) };
            if success.is_err() { break; }
        }
    }

    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  enumerate_subdirectories
//
//  Enumerate subdirectories and enqueue them as children.
//
//  Port of: CMultiThreadedLister::EnumerateSubdirectories
//
////////////////////////////////////////////////////////////////////////////////

fn enumerate_subdirectories(
    node: &WorkItem,
    work_queue: &WorkQueue<WorkItem>,
    stop: &AtomicBool,
    tree_pruning_active: &AtomicBool,
    cmd: &CommandLine,
) -> Result<(), String> {
    let (dir_path, file_specs) = {
        let di = node.0.lock().unwrap();
        (di.dir_path.clone(), di.file_specs.clone())
    };

    //
    // In tree mode, directories must also appear in matches so they are
    // visible in the interleaved tree display.  Build a set of directory
    // names already present (from enumerate_matching_files) to avoid
    // duplicates.
    //

    let is_tree = cmd.tree.unwrap_or (false);
    let mut seen_dirs: HashSet<String> = HashSet::new();

    if is_tree {
        let di = node.0.lock().unwrap();
        for entry in &di.matches {
            if (entry.file_attributes & FILE_ATTRIBUTE_DIRECTORY) != 0 {
                seen_dirs.insert (entry.file_name.to_string_lossy().to_lowercase());
            }
        }
    }

    let mut search_path = dir_path.clone();
    search_path.push("*");
    let search_wide: Vec<u16> = search_path.as_os_str().encode_wide().chain(Some(0)).collect();

    let mut wfd = WIN32_FIND_DATAW::default();
    let handle = unsafe { FindFirstFileW(windows::core::PCWSTR(search_wide.as_ptr()), &mut wfd) };
    let handle = match handle {
        Ok(h) if !h.is_invalid() => h,
        _ => return Ok(()),
    };
    let _find_handle = FindHandle(handle);

    loop {
        if stop.load(Ordering::Acquire) { break; }

        if !is_dots(&wfd.cFileName)
            && (wfd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0
        {
            let name_len = wfd.cFileName.iter().position(|&c| c == 0).unwrap_or(0);
            let name = OsString::from_wide(&wfd.cFileName[..name_len]);
            let subdir_path = dir_path.join(&name);

            let mut child_di = DirectoryInfo::new_multi (subdir_path, file_specs.clone());

            // Set parent weak reference for tree pruning propagation
            if tree_pruning_active.load (Ordering::Acquire) {
                child_di.parent = Some (Arc::downgrade (node));
            }

            let child_node: WorkItem = Arc::new ((Mutex::new (child_di), Condvar::new()));

            // Add child to parent's children list
            {
                let mut di = node.0.lock().unwrap();
                di.children.push(Arc::clone(&child_node));
            }

            work_queue.push(Arc::clone (&child_node));

            //
            // In tree mode, add every directory to matches so the tree
            // display can show it and recurse into it.  Skip if already
            // added by enumerate_matching_files.
            //

            if is_tree {
                let lower_name = name.to_string_lossy().to_lowercase();

                if !seen_dirs.contains (&lower_name) {
                    seen_dirs.insert (lower_name);
                    let file_entry = FileInfo::from_find_data (&wfd);
                    let mut di = node.0.lock().unwrap();
                    add_match_to_list (&wfd, file_entry, &mut di, cmd);
                }
            }
        }

        let success = unsafe { FindNextFileW(handle, &mut wfd) };
        if success.is_err() { break; }
    }

    Ok(())
}





////////////////////////////////////////////////////////////////////////////////
//
//  add_match_to_list
//
//  Add a matched file entry to a DirectoryInfo node.
//
////////////////////////////////////////////////////////////////////////////////

fn add_match_to_list(wfd: &WIN32_FIND_DATAW, file_entry: FileInfo, di: &mut DirectoryInfo, cmd: &CommandLine) {
    // Track filename length for wide listing
    if cmd.wide_listing {
        let name_len = wfd.cFileName.iter().position(|&c| c == 0).unwrap_or(0);
        let len = if (wfd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0 {
            name_len + 2
        } else {
            name_len
        };
        if len > di.largest_file_name {
            di.largest_file_name = len;
        }
    }

    if (wfd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0 {
        di.subdirectory_count += 1;
    } else {
        let file_size = file_entry.file_size;
        if file_size > di.largest_file_size {
            di.largest_file_size = file_size;
        }
        di.bytes_used += file_size;
        di.file_count += 1;
        // Note: totals not tracked per-node in MT mode — accumulated by consumer
    }

    di.matches.push(file_entry);
}





////////////////////////////////////////////////////////////////////////////////
//
//  wait_for_node_completion
//
//  Wait for a directory node to complete enumeration.
//
//  Port of: CMultiThreadedLister::WaitForNodeCompletion
//
////////////////////////////////////////////////////////////////////////////////

fn wait_for_node_completion(
    node: &WorkItem,
    stop: &AtomicBool,
) -> (DirectoryStatus, Option<String>) {
    let (ref mutex, ref condvar) = **node;
    let mut di = mutex.lock().unwrap();

    while di.status != DirectoryStatus::Done
       && di.status != DirectoryStatus::Error
       && !stop.load(Ordering::Acquire)
    {
        di = condvar.wait(di).unwrap();
    }

    let status = di.status;
    let error = di.error.clone();
    (status, error)
}





////////////////////////////////////////////////////////////////////////////////
//
//  accumulate_totals
//
//  Accumulate totals from a completed node.
//
//  Port of: CMultiThreadedLister::AccumulateTotals
//
////////////////////////////////////////////////////////////////////////////////

fn accumulate_totals(di: &DirectoryInfo, totals: &mut ListingTotals) {
    totals.file_count      += di.file_count;
    totals.file_bytes      += di.bytes_used;
    totals.stream_count    += di.stream_count;
    totals.stream_bytes    += di.stream_bytes_used;
    totals.directory_count += di.subdirectory_count;
}





////////////////////////////////////////////////////////////////////////////////
//
//  is_dots
//
//  Check if a filename is "." or ".."
//
////////////////////////////////////////////////////////////////////////////////

fn is_dots(filename: &[u16]) -> bool {
    if filename[0] == b'.' as u16 {
        if filename[1] == 0 {
            return true;
        }
        if filename[1] == b'.' as u16 && filename[2] == 0 {
            return true;
        }
    }
    false
}





////////////////////////////////////////////////////////////////////////////////
//
//  propagate_descendant_match
//
//  Walk up the parent chain, setting descendant_match_found on each
//  ancestor.  Short-circuits if an ancestor already has the flag set
//  (meaning all further ancestors were already notified).
//
//  Port of: CMultiThreadedLister::PropagateDescendantMatch
//
////////////////////////////////////////////////////////////////////////////////

fn propagate_descendant_match (parent_weak: &Weak<(Mutex<DirectoryInfo>, Condvar)>) {
    let mut current = parent_weak.upgrade();

    while let Some (ancestor) = current {
        let already_set = {
            let mut di = ancestor.0.lock().unwrap();
            let was_set = di.descendant_match_found;
            di.descendant_match_found = true;
            was_set
        };

        // Notify any consumer waiting on this node
        ancestor.1.notify_all();

        // Short-circuit: if already set, all further ancestors were too
        if already_set {
            break;
        }

        // Move up to grandparent
        let next_parent = {
            let di = ancestor.0.lock().unwrap();
            match di.parent {
                Some (ref w) => w.upgrade(),
                None         => None,
            }
        };
        current = next_parent;
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  signal_subtree_complete
//
//  Mark this node's subtree as complete and propagate upward.  A parent's
//  subtree is complete when ALL of its children are subtree_complete.
//
//  Port of: CMultiThreadedLister::TrySignalParentSubtreeComplete
//
////////////////////////////////////////////////////////////////////////////////

fn signal_subtree_complete (node: &WorkItem) {
    // Mark this node as subtree_complete
    {
        let mut di = node.0.lock().unwrap();
        di.subtree_complete = true;
    }
    node.1.notify_all();

    // Walk upward: check if the parent's subtree is now fully complete
    let parent_ref = {
        let di = node.0.lock().unwrap();
        di.parent.as_ref().and_then (|w| w.upgrade())
    };

    if let Some (parent_node) = parent_ref {
        let all_children_complete = {
            let parent_di = parent_node.0.lock().unwrap();

            // Parent must itself be Done for its subtree to be complete
            if parent_di.status != DirectoryStatus::Done {
                return;
            }

            parent_di.children.iter().all (|child| {
                let child_di = child.0.lock().unwrap();
                child_di.subtree_complete
            })
        };

        if all_children_complete {
            signal_subtree_complete (&parent_node);
        }
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  wait_for_tree_visibility
//
//  Block until we know whether a child directory should be displayed:
//  - If descendant_match_found becomes true → display it (return true)
//  - If subtree_complete becomes true without matches → prune (return false)
//
//  Port of: CMultiThreadedLister::WaitForTreeVisibility
//
////////////////////////////////////////////////////////////////////////////////

fn wait_for_tree_visibility (child_node: &WorkItem, stop: &AtomicBool) -> bool {
    // Quick-path: check without condvar wait
    {
        let di = child_node.0.lock().unwrap();
        if di.descendant_match_found {
            return true;
        }
        if di.subtree_complete {
            return false;
        }
    }

    // Slow-path: wait for a signal
    let (ref mutex, ref condvar) = **child_node;
    let mut di = mutex.lock().unwrap();

    loop {
        if di.descendant_match_found {
            return true;
        }
        if di.subtree_complete {
            return false;
        }
        if stop.load (Ordering::Acquire) {
            return false;
        }
        di = condvar.wait (di).unwrap();
    }
}





#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_dots_single_dot
    //
    //  Verify "." is detected as a dot entry.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn is_dots_single_dot() {
        let name = [b'.' as u16, 0, 0, 0];
        assert!(is_dots(&name));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_dots_double_dot
    //
    //  Verify ".." is detected as a dot entry.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn is_dots_double_dot() {
        let name = [b'.' as u16, b'.' as u16, 0, 0];
        assert!(is_dots(&name));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_dots_regular_file
    //
    //  Verify a regular file name is not a dot entry.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn is_dots_regular_file() {
        let name: Vec<u16> = "hello.txt\0".encode_utf16().collect();
        assert!(!is_dots(&name));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  is_dots_dot_prefix
    //
    //  Verify ".gitignore" is not detected as a dot entry.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn is_dots_dot_prefix() {
        let name: Vec<u16> = ".gitignore\0".encode_utf16().collect();
        assert!(!is_dots(&name));
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  accumulate_totals_adds
    //
    //  Verify accumulate_totals correctly sums counts and byte totals.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn accumulate_totals_adds() {
        let mut di = DirectoryInfo::new (PathBuf::from ("C:\\test"), "*".into());
        di.file_count         = 5;
        di.subdirectory_count = 2;
        di.bytes_used         = 1024;
        di.stream_count       = 1;
        di.stream_bytes_used  = 256;

        let mut totals = ListingTotals::default();
        accumulate_totals (&di, &mut totals);

        assert_eq!(totals.file_count,      5);
        assert_eq!(totals.directory_count,  2);
        assert_eq!(totals.file_bytes,       1024);
        assert_eq!(totals.stream_count,     1);
        assert_eq!(totals.stream_bytes,     256);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  accumulate_totals_multiple
    //
    //  Verify accumulate_totals accumulates across multiple directory infos.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn accumulate_totals_multiple() {
        let mut totals = ListingTotals::default();

        let mut di1 = DirectoryInfo::new (PathBuf::from ("C:\\a"), "*".into());
        di1.file_count = 3;
        di1.bytes_used = 500;
        accumulate_totals (&di1, &mut totals);

        let mut di2 = DirectoryInfo::new (PathBuf::from ("C:\\b"), "*".into());
        di2.file_count = 7;
        di2.bytes_used = 1500;
        accumulate_totals (&di2, &mut totals);

        assert_eq!(totals.file_count, 10);
        assert_eq!(totals.file_bytes, 2000);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  wait_for_done_node
    //
    //  Verify wait_for_node_completion returns immediately for Done nodes.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn wait_for_done_node() {
        let mut di = DirectoryInfo::new (PathBuf::from ("C:\\test"), "*".into());
        di.status = DirectoryStatus::Done;

        let node: WorkItem = Arc::new ((Mutex::new (di), Condvar::new()));
        let stop = AtomicBool::new (false);

        let (status, error) = wait_for_node_completion (&node, &stop);
        assert_eq!(status, DirectoryStatus::Done);
        assert!(error.is_none());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  wait_for_error_node
    //
    //  Verify wait_for_node_completion returns error info for Error nodes.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn wait_for_error_node() {
        let mut di = DirectoryInfo::new (PathBuf::from ("C:\\bad"), "*".into());
        di.status = DirectoryStatus::Error;
        di.error  = Some ("access denied".into());

        let node: WorkItem = Arc::new ((Mutex::new (di), Condvar::new()));
        let stop = AtomicBool::new (false);

        let (status, error) = wait_for_node_completion (&node, &stop);
        assert_eq!(status, DirectoryStatus::Error);
        assert_eq!(error.unwrap(), "access denied");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  wait_for_node_with_thread_completion
    //
    //  Verify wait_for_node_completion blocks until a worker signals Done.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn wait_for_node_with_thread_completion() {
        let di = DirectoryInfo::new (PathBuf::from ("C:\\test"), "*".into());
        let node: WorkItem = Arc::new ((Mutex::new (di), Condvar::new()));
        let stop = Arc::new (AtomicBool::new (false));

        // Spawn a thread that completes the node after a short delay
        let node_clone = Arc::clone (&node);
        let worker = thread::spawn (move || {
            thread::sleep (std::time::Duration::from_millis (50));
            {
                let mut di = node_clone.0.lock().unwrap();
                di.status = DirectoryStatus::Done;
                di.file_count = 42;
            }
            node_clone.1.notify_one();
        });

        let (status, _error) = wait_for_node_completion (&node, &stop);
        assert_eq!(status, DirectoryStatus::Done);

        // Verify the data is there
        let di = node.0.lock().unwrap();
        assert_eq!(di.file_count, 42);

        worker.join().unwrap();
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  wait_for_node_stop_requested
    //
    //  Verify wait_for_node_completion returns early when stop is signaled.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn wait_for_node_stop_requested() {
        let di = DirectoryInfo::new (PathBuf::from ("C:\\test"), "*".into());
        let node: WorkItem = Arc::new ((Mutex::new (di), Condvar::new()));
        let stop = Arc::new (AtomicBool::new (false));

        // Spawn a thread that signals stop after a delay (instead of completing the node)
        let stop_clone = Arc::clone (&stop);
        let node_clone = Arc::clone (&node);
        let signaler = thread::spawn (move || {
            thread::sleep (std::time::Duration::from_millis (50));
            stop_clone.store (true, Ordering::Release);
            node_clone.1.notify_one();
        });

        let (status, _error) = wait_for_node_completion (&node, &stop);
        // Node is still Waiting because nobody completed it — stop was signaled
        assert_eq!(status, DirectoryStatus::Waiting);

        signaler.join().unwrap();
    }
}
