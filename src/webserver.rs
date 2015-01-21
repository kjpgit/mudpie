use std::sync::Arc;
use std::io::{TcpListener, TcpStream};
use std::io::net::tcp::TcpAcceptor;
use std::io::{Acceptor, Listener};
use std::thread::Thread;

pub use request::WebRequest;

use threadpool::ThreadPool;
use request;
use utils;

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


pub type PageFunction = fn(&WebRequest) -> WebResponse;

struct DispatchRule {
    prefix: String,
    page_fn: PageFunction
}

struct WorkerSharedContext {
    rules: Vec<DispatchRule>,
    acceptor: TcpAcceptor,
}

struct WorkerPrivateContext {
    thread_id: i64,
    shared_ctx: Arc<WorkerSharedContext>,
}


pub struct WebResponse {
    data: Vec<u8>,
    content_type: String
}

impl WebResponse {
    /// Shortcut for creating a successful Unicode HTML response.
    ///
    /// * response code: 200
    /// * content-type: "text/html; charset=utf-8"
    pub fn new_html(body: String) -> WebResponse {
        return WebResponse{
                data: body.into_bytes(),
                content_type: "text/html; charset=utf-8".to_string()
            };
    }
}

pub struct WebServer {
    rules: Option<Vec<DispatchRule>>,
    thread_pool: ThreadPool,
    worker_shared_context: Option<Arc<WorkerSharedContext>>,
}

impl WebServer {
    pub fn new() -> WebServer {
        let ret = WebServer{
                rules: Some(Vec::new()),
                thread_pool: ThreadPool::new(),
                worker_shared_context: None,
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

    /// Starts `num_threads` worker threads.  If any fail, they will be
    /// respawned.  This function does not return.
    pub fn run(&mut self, address: &str, port: i32, num_threads: i32) {
        let addr = format!("{}:{}", address, port);
        let listener = TcpListener::bind(addr.as_slice());
        let mut acceptor = listener.listen().unwrap();
        
        // .clone doesn't work, compiler bug
        let mut page_fn_copy = self.rules.take().unwrap();

        let ctx = WorkerSharedContext {
            rules: page_fn_copy,
            acceptor: acceptor,
        };
        self.worker_shared_context = Some(Arc::new(ctx));

        for i in range(0, num_threads) {
            self.start_new_worker();
        }
        loop {
            self.thread_pool.wait_for_thread_exit();
            println!("uh oh, a worker thread died");
            println!("starting another worker");
            self.start_new_worker();
        }
    }

    fn start_new_worker(&mut self) {
        let priv_ctx = WorkerPrivateContext {
            thread_id: 0i64,
            shared_ctx: self.worker_shared_context.as_mut().unwrap().clone(),
        };
        self.thread_pool.execute(move || {
            worker_thread_main(priv_ctx);
        });
    }
}


fn worker_thread_main(ctx: WorkerPrivateContext) {
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


// HTTP specific parsing/errors

fn process_http_connection(ctx: &WorkerPrivateContext, stream: TcpStream) {
    let mut sentinel = HTTPContext { 
        stream: stream, 
        started_response: false 
    };
    let req = read_request(&mut sentinel.stream);
    for rule in ctx.shared_ctx.rules.iter() {
        // todo: prefix
        if rule.prefix == req.path {
            let response = (rule.page_fn)(&req);
            sentinel.send_response(200, "OK DOKIE",
                    response.data.as_slice());
            return;
        }
    }
    sentinel.send_response(404, "Not Found, Bro", 
        b"Resource not found");
}

struct HTTPContext {
    stream: TcpStream,
    started_response: bool,
}

impl HTTPContext {
    fn send_response(&mut self, code: i32, 
            status: &str, body: &[u8]) {
        let mut headers = String::new();
        headers.push_str(format!("HTTP/1.1 {} {}\r\n", code, status).as_slice());
        headers.push_str("Connection: close\r\n");
        headers.push_str("\r\n");

        // todo: error check
        self.started_response = true;
        self.stream.write_str(headers.as_slice());
        self.stream.write(body);
    }
}

impl Drop for HTTPContext {
    /// If we paniced and/or are about to die, make sure client gets a 500
    fn drop(&mut self) {
        if !self.started_response {
            self.send_response(500, 
                "Uh oh :-(", 
                b"Internal Error");
        }
    }
}

fn read_request(stream: &mut TcpStream) -> WebRequest {
    let mut req_buffer = Vec::<u8>::new();
    let mut chunk_buffer = [0; 1024];  // todo: move to heap
    loop {
        let ret = stream.read(&mut chunk_buffer);
        let size = ret.unwrap();
        //println!("size {}", size);
        if size > 0 {
            req_buffer.extend(chunk_buffer.slice(0, size).iter().cloned());
            //println!("req_buffer {}", req_buffer.len());
            let split_pos = utils::memmem(req_buffer.as_slice(), b"\r\n\r\n");
            if split_pos.is_some() {
                let split_pos = split_pos.unwrap();
                println!("split pos: {}", split_pos);
                if split_pos >= 0 {
                    return request::parse_request(req_buffer.as_slice());
                }
            }
        }
    }
}
