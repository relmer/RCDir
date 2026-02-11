// work_queue.rs — Thread-safe work queue for parallel enumeration
//
// Port of: WorkQueue.h → CWorkQueue<T>
//
// A simple MPMC FIFO queue with blocking pop and done signaling.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};





/// Thread-safe FIFO work queue.
///
/// Port of: CWorkQueue<T>
pub struct WorkQueue<T> {
    inner: Mutex<WorkQueueInner<T>>,
    cv:    Condvar,
}





struct WorkQueueInner<T> {
    queue: VecDeque<T>,
    done:  bool,
}





impl<T> Default for WorkQueue<T> {
    ////////////////////////////////////////////////////////////////////////////
    //
    //  default
    //
    //  Default constructor — delegates to new().
    //
    ////////////////////////////////////////////////////////////////////////////

    fn default() -> Self {
        Self::new()
    }
}





impl<T> WorkQueue<T> {
    ////////////////////////////////////////////////////////////////////////////
    //
    //  new
    //
    //  Create a new empty WorkQueue.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn new() -> Self {
        WorkQueue {
            inner: Mutex::new(WorkQueueInner {
                queue: VecDeque::new(),
                done:  false,
            }),
            cv: Condvar::new(),
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  push
    //
    //  Push an item onto the queue. Ignored if done.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn push(&self, item: T) {
        let mut inner = self.inner.lock().unwrap();

        if !inner.done {
            inner.queue.push_back(item);
            self.cv.notify_one();
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  pop
    //
    //  Pop an item from the queue. Blocks until available or done.
    //  Returns None if queue is empty and done.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn pop(&self) -> Option<T> {
        let mut inner = self.inner.lock().unwrap();

        loop {
            if let Some(item) = inner.queue.pop_front() {
                return Some(item);
            }

            if inner.done {
                return None;
            }

            inner = self.cv.wait(inner).unwrap();
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  set_done
    //
    //  Signal that no more items will be pushed.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn set_done(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.done = true;
        self.cv.notify_all();
    }
}





#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    ////////////////////////////////////////////////////////////////////////////
    //
    //  push_pop_basic
    //
    //  Basic push and pop operations.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn push_pop_basic() {
        let q = WorkQueue::new();
        q.push(1);
        q.push(2);
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.pop(), Some(2));
        q.set_done();
        assert_eq!(q.pop(), None);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  pop_blocks_until_push
    //
    //  Pop should block until an item is pushed.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn pop_blocks_until_push() {
        let q = Arc::new(WorkQueue::new());
        let q2 = Arc::clone(&q);

        let handle = thread::spawn(move || {
            q2.pop()
        });

        // Small delay so the thread blocks
        thread::sleep(std::time::Duration::from_millis(10));
        q.push(42);

        assert_eq!(handle.join().unwrap(), Some(42));
        q.set_done();
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  pop_returns_none_when_done_empty
    //
    //  Pop returns None when the queue is done and empty.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn pop_returns_none_when_done_empty() {
        let q: WorkQueue<i32> = WorkQueue::new();
        q.set_done();
        assert_eq!(q.pop(), None);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  push_after_done_is_ignored
    //
    //  Pushing after set_done should be silently ignored.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn push_after_done_is_ignored() {
        let q = WorkQueue::new();
        q.set_done();
        q.push(99);
        assert_eq!(q.pop(), None);
    }
}
