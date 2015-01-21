//! Simple generic thread pool

use std::sync::{Arc,Condvar,Mutex};
use std::thread::Thread;


/// Like Taskpool, but delegates recovery to a main thread.
///
/// Designed to be very resilient on thread panic - no memory allocations are
/// done, just a condvar is signaled.  Worker threads are detached, so they
/// don't need to be joined on failure to release resources.
///
pub struct ThreadPool {
    shared_ctx: Arc<ThreadPoolShared>,
}

// Shared between all workers and the main thread
struct ThreadPoolShared {
    // Increments on each thread panic
    watchdog_mutex: Mutex<i64>,
    watchdog_cvar: Condvar,
}


impl ThreadPool {
    pub fn new() -> ThreadPool {
        let ctx = ThreadPoolShared {
            watchdog_mutex: Mutex::new(0),
            watchdog_cvar: Condvar::new(),
        };
        let ret = ThreadPool {
            shared_ctx: Arc::new(ctx),
        };
        return ret;
    }

    /// Spawn a detached worker thread that executes a job function once.
    ///
    /// If the thread panics, it will not be respawned.
    pub fn execute<F: FnOnce() + Send>(&mut self, job: F) {
        let ctx = self.shared_ctx.clone();
        Thread::spawn(move || {
            let _sentinel = WorkerSentinel { ctx: ctx };
            job();
        });
    }

    /// Returns if a worker thread terminates (e.g. from panic).
    ///
    /// You can call this in a loop and respawn threads as needed.
    /// You might want to add a teen-tiny sleep delay because we have no way to
    /// join() on the dead thread, so thread handles could pile up in theory if
    /// respawning runs away.
    pub fn wait_for_thread_exit(&mut self) {
        let mut guard = self.shared_ctx.watchdog_mutex.lock().unwrap();
        loop {
            if *guard > 0 {
                *guard -= 1;
                return;
            } else {
                guard = self.shared_ctx.watchdog_cvar.wait(guard).unwrap();
            }
        }
    }
}


struct WorkerSentinel {
    ctx: Arc<ThreadPoolShared>,
}

impl Drop for WorkerSentinel {
    fn drop(&mut self) {
        // ruh roh! alert master!
        // todo: error check?
        let mut lock = self.ctx.watchdog_mutex.lock().unwrap();
        *lock += 1;
        self.ctx.watchdog_cvar.notify_one();
    }
}
