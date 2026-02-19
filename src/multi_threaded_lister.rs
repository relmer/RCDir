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
use crate::results_displayer::{DirectoryLevel, Displayer, ResultsDisplayer};
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
        let root = DirectoryInfo::new_multi(dir_path.to_path_buf(), spec_strings);
        let root_node: WorkItem = Arc::new((Mutex::new(root), Condvar::new()));

        self.work_queue.push(Arc::clone(&root_node));

        // Consume the tree on the main thread (streaming output)
        self.print_directory_tree(&root_node, drive_info, displayer, DirectoryLevel::Initial, totals);
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
    cmd: &CommandLine,
) -> Result<(), String> {
    enumerate_matching_files(node, stop, cmd)?;

    if cmd.recurse {
        enumerate_subdirectories(node, work_queue, stop)?;
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
