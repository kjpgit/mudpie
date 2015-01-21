use std::collections::HashMap;
use std::sync::Arc;
use std::io::{TcpListener, TcpStream};
use std::io::net::tcp::TcpAcceptor;
use std::io::{Acceptor, Listener};
use std::thread::Thread;
use std::sync::{Condvar,Mutex};
use std::os::unix::prelude;

/*

Main thread:
- creates listening socket
- starts worker threads
- waits on worker_monitor condition var, to see when threads need respawn

Worker threads:
- clone the listening socket, so they can each call accept().  no context
  switches needed.
- passed the global configuraiton and dispatch map (read only)

Note that for linux kernel >= 3.9, an optimization (for VERY high req/sec
loads) is to use SO_REUSEPORT so each worker thread has a different socket.
That would be a trivial change, and not affect this architecture.

*/


type PageFunction = fn(&WebRequest) -> WebResponse;

struct DispatchRule {
    prefix: String,
    page_fn: PageFunction
}

struct WorkerSharedContext {
    rules: Vec<DispatchRule>,
    acceptor: TcpAcceptor,

    // for thread restarting
    watchdog_mutex: Mutex<i64>,
    watchdog_cvar: Condvar,
}

struct WorkerPrivateContext {
    thread_id: i64,
    shared_ctx: Arc<WorkerSharedContext>,
}

pub struct WebRequest { 
    header: bool,
}

pub struct WebResponse {
    data: Vec<u8>,
    content_type: String
}

impl WebResponse {
    pub fn new_html(body: String) -> WebResponse {
        return WebResponse{
                data: body.into_bytes(),
                content_type: "text/html; charset=utf-8".to_string()
            };
    }
}

pub struct WebServer {
    rules: Option<Vec<DispatchRule>>,
}

impl WebServer {
    pub fn new() -> WebServer {
        let ret = WebServer{
                rules: Some(Vec::new()),
                };
        return ret;
    }

    pub fn add_path(&mut self, path: &str, page_fn: PageFunction) {
        let fn_map = self.rules.as_mut().unwrap();
        let rule = DispatchRule { 
            prefix: path.to_string(), 
            page_fn: page_fn 
        };
        fn_map.push(rule);
    }

    pub fn run_legacy(&mut self, address: &str, port: i32, num_threads: i32) {
        let addr = format!("{}:{}", address, port);
        let listener = TcpListener::bind(addr.as_slice());
        let mut acceptor = listener.listen().unwrap();
        
        // .clone doesn't work, compiler bug
        let mut page_fn_copy = self.rules.take().unwrap();

        let ctx = WorkerSharedContext {
            rules: page_fn_copy,
            acceptor: acceptor,
            watchdog_mutex: Mutex::new(0),
            watchdog_cvar: Condvar::new(),
        };
        let ctx = Arc::new(ctx);

        for i in range(0, num_threads) {
            let priv_ctx = WorkerPrivateContext {
                thread_id: i as i64,
                shared_ctx: ctx.clone(),
            };
            let handle = Thread::spawn(move ||
                    worker_thread_main(priv_ctx)
                );
        }

        let mut guard = ctx.watchdog_mutex.lock().unwrap();
        loop {
            if *guard > 0 {
                println!("uh oh, a worker thread died!");
                *guard -= 1;
            } else {
                println!("monitoring worker threads");
                guard = ctx.watchdog_cvar.wait(guard).unwrap();
            }
        }
    }
}


fn worker_thread_main(ctx: WorkerPrivateContext) {
    let sentinel = WorkerSentinel { ctx: &ctx };

    println!("worker thread started: {}", ctx.thread_id);
    let mut acceptor = ctx.shared_ctx.acceptor.clone();
    loop {
        let mut res = acceptor.accept();
        match res {
            Ok(sock) => process_http_connection(&ctx, sock),
            Err(err) => println!("error :-( {}", err)
        }
    }
}

struct WorkerSentinel<'a> {
    ctx: &'a WorkerPrivateContext,
}

#[unsafe_destructor]
impl<'a> Drop for WorkerSentinel<'a> {
    fn drop(&mut self) {
        // ruh roh! alert master!
        // todo: error check?
        let mut lock = self.ctx.shared_ctx.watchdog_mutex.lock().unwrap();
        *lock += 1;
        self.ctx.shared_ctx.watchdog_cvar.notify_one();
    }
}


// HTTP specific parsing/errors

fn process_http_connection(ctx: &WorkerPrivateContext, stream: TcpStream) {
    let mut sentinel = HTTPContext{ stream: stream, started_response: false };
    //let data = sentinel.stream.read_to_end();
    let ref pfn = ctx.shared_ctx.rules[0];
    let req = WebRequest { header: false };
    let response = (pfn.page_fn)(&req);
    sentinel.send_response(200, "OK DOKIE",
            &response.data);
}

struct HTTPContext {
    stream: TcpStream,
    started_response: bool,
}

impl HTTPContext {
    fn send_response(&mut self, code: i32, 
            status: &str, body: &Vec<u8>) {
        let mut headers = String::new();
        headers.push_str(format!("HTTP/1.1 {} {}\r\n", code, status).as_slice());
        headers.push_str("Connection: close\r\n");
        headers.push_str("\r\n");

        // todo: error check
        self.started_response = true;
        self.stream.write_str(headers.as_slice());
        self.stream.write(body.as_slice());
    }
}

impl Drop for HTTPContext {
    /// If we paniced and/or are about to die, make sure client gets a 500
    fn drop(&mut self) {
        if !self.started_response {
            self.send_response(500, 
                "Uh oh :-(", 
                &"Internal Error".to_string().into_bytes());
        }
    }
}