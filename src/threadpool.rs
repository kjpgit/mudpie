use std::sync::{Arc,Condvar,Mutex};
use std::thread::Thread;


pub struct ThreadPool {
    shared_ctx: Arc<ThreadPoolShared>,
}

// Shared between all workers and the main thread
struct ThreadPoolShared {
    // For thread restarting
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

    pub fn execute<F: FnOnce() + Send>(&mut self, job: F) {
        let ctx = self.shared_ctx.clone();
        Thread::spawn(move || {
            let sentinel = WorkerSentinel { ctx: ctx };
            job();
        });
    }

    pub fn wait_for_thread_exit(&mut self) {
        let mut guard = self.shared_ctx.watchdog_mutex.lock().unwrap();
        loop {
            if *guard > 0 {
                // todo: join it
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
