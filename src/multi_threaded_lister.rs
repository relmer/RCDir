// multi_threaded_lister.rs — Multi-threaded recursive directory enumeration
//
// Port of: MultiThreadedLister.h, MultiThreadedLister.cpp
//
// Producer-consumer pattern: worker threads enumerate directories in parallel,
// main thread walks the tree depth-first for in-order streaming output.

use std::collections::HashSet;
use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
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
use crate::file_info::{FileInfo, FindHandle, FILE_ATTRIBUTE_DIRECTORY};
use crate::listing_totals::ListingTotals;
use crate::results_displayer::{DirectoryLevel, NormalDisplayer, ResultsDisplayer};
use crate::work_queue::WorkQueue;

/// A work item is a reference to a directory node in the tree.
type WorkItem = Arc<(Mutex<DirectoryInfo>, Condvar)>;

/// Multi-threaded recursive directory lister.
///
/// Port of: CMultiThreadedLister
pub struct MultiThreadedLister {
    cmd:        Arc<CommandLine>,
    _config:    Arc<Config>,
    work_queue: Arc<WorkQueue<WorkItem>>,
    stop:       Arc<AtomicBool>,
    workers:    Vec<JoinHandle<()>>,
}

impl MultiThreadedLister {
    /// Create a new multi-threaded lister and spawn worker threads.
    pub fn new(cmd: Arc<CommandLine>, config: Arc<Config>) -> Self {
        let work_queue = Arc::new(WorkQueue::new());
        let stop = Arc::new(AtomicBool::new(false));

        let num_threads = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .max(1);

        let mut workers = Vec::with_capacity(num_threads);

        for _ in 0..num_threads {
            let wq = Arc::clone(&work_queue);
            let st = Arc::clone(&stop);
            let c  = Arc::clone(&cmd);
            let cf = Arc::clone(&config);

            workers.push(thread::spawn(move || {
                worker_thread_func(&wq, &st, &c, &cf);
            }));
        }

        MultiThreadedLister { cmd, _config: config, work_queue, stop, workers }
    }

    /// Process a directory tree with multi-threaded enumeration.
    ///
    /// Port of: CMultiThreadedLister::ProcessDirectoryMultiThreaded
    pub fn process(
        &mut self,
        drive_info: &DriveInfo,
        dir_path: &Path,
        file_specs: &[OsString],
        displayer: &mut NormalDisplayer,
        totals: &mut ListingTotals,
    ) {
        let spec_strings: Vec<String> = file_specs.iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        let root = DirectoryInfo::new_multi(dir_path.to_path_buf(), spec_strings);
        let root_node: WorkItem = Arc::new((Mutex::new(root), Condvar::new()));

        self.work_queue.push(Arc::clone(&root_node));

        // Consume the tree on the main thread (streaming output)
        self.print_directory_tree(&root_node, drive_info, displayer, DirectoryLevel::Initial, totals);
    }

    /// Stop all worker threads and join them.
    pub fn stop_workers(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.work_queue.set_done();

        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }

    /// Recursive depth-first tree walk — consumes results in discovery order.
    ///
    /// Port of: CMultiThreadedLister::PrintDirectoryTree
    fn print_directory_tree(
        &self,
        node: &WorkItem,
        drive_info: &DriveInfo,
        displayer: &mut NormalDisplayer,
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

            file_comparator::sort_files(&mut di.matches, &self.cmd);
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

    fn stop_requested(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }
}

impl Drop for MultiThreadedLister {
    fn drop(&mut self) {
        self.stop_workers();
    }
}

// ── Worker thread ─────────────────────────────────────────────────────────────

/// Worker thread function — processes items from the work queue.
///
/// Port of: CMultiThreadedLister::WorkerThreadFunc
fn worker_thread_func(
    work_queue: &WorkQueue<WorkItem>,
    stop: &AtomicBool,
    cmd: &CommandLine,
    _config: &Config,
) {
    while !stop.load(Ordering::Acquire) {
        let item = match work_queue.pop() {
            Some(item) => item,
            None => break, // Queue is done
        };

        enumerate_directory_node(&item, work_queue, stop, cmd);
    }
}

/// Enumerate a single directory node (producer function).
///
/// Port of: CMultiThreadedLister::EnumerateDirectoryNode
fn enumerate_directory_node(
    node: &WorkItem,
    work_queue: &WorkQueue<WorkItem>,
    stop: &AtomicBool,
    cmd: &CommandLine,
) {
    // Set InProgress
    {
        let mut di = node.0.lock().unwrap();
        di.status = DirectoryStatus::InProgress;
    }

    let result = perform_enumeration(node, work_queue, stop, cmd);

    // Set Done or Error
    {
        let mut di = node.0.lock().unwrap();
        match result {
            Ok(()) => di.status = DirectoryStatus::Done,
            Err(msg) => {
                di.status = DirectoryStatus::Error;
                di.error = Some(msg);
            }
        }
    }

    // Notify consumer
    node.1.notify_one();
}

/// Perform the actual enumeration: matching files + subdirectories.
///
/// Port of: CMultiThreadedLister::PerformEnumeration
fn perform_enumeration(
    node: &WorkItem,
    work_queue: &WorkQueue<WorkItem>,
    stop: &AtomicBool,
    cmd: &CommandLine,
) -> Result<(), String> {
    enumerate_matching_files(node, stop, cmd)?;

    if cmd.recurse {
        enumerate_subdirectories(node, work_queue, stop)?;
    }

    Ok(())
}

/// Enumerate files matching the file specs, with deduplication across specs.
///
/// Port of: CMultiThreadedLister::EnumerateMatchingFiles
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

/// Enumerate subdirectories and enqueue them as children.
///
/// Port of: CMultiThreadedLister::EnumerateSubdirectories
fn enumerate_subdirectories(
    node: &WorkItem,
    work_queue: &WorkQueue<WorkItem>,
    stop: &AtomicBool,
) -> Result<(), String> {
    let (dir_path, file_specs) = {
        let di = node.0.lock().unwrap();
        (di.dir_path.clone(), di.file_specs.clone())
    };

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

            let child_di = DirectoryInfo::new_multi(subdir_path, file_specs.clone());
            let child_node: WorkItem = Arc::new((Mutex::new(child_di), Condvar::new()));

            // Add child to parent's children list
            {
                let mut di = node.0.lock().unwrap();
                di.children.push(Arc::clone(&child_node));
            }

            work_queue.push(child_node);
        }

        let success = unsafe { FindNextFileW(handle, &mut wfd) };
        if success.is_err() { break; }
    }

    Ok(())
}

// ── Helper functions ──────────────────────────────────────────────────────────

/// Add a matched file entry to a DirectoryInfo node.
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

/// Wait for a directory node to complete enumeration.
///
/// Port of: CMultiThreadedLister::WaitForNodeCompletion
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

/// Accumulate totals from a completed node.
///
/// Port of: CMultiThreadedLister::AccumulateTotals
fn accumulate_totals(di: &DirectoryInfo, totals: &mut ListingTotals) {
    totals.file_count      += di.file_count;
    totals.file_bytes      += di.bytes_used;
    totals.stream_count    += di.stream_count;
    totals.stream_bytes    += di.stream_bytes_used;
    totals.directory_count += di.subdirectory_count;
}

/// Check if a filename is "." or ".."
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
