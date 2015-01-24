use std::collections::HashMap;
use std::sync::Arc;
use std::io::{TcpListener, TcpStream};
use std::io::net::tcp::TcpAcceptor;
use std::io::{Acceptor, Listener};

use utils::threadpool::ThreadPool;
use utils::byteutils;
use utils::http_request;


/// A response that will be sent to the client (code, headers, body)
pub struct WebResponse {
    code: i32,
    status: String, 
    data: Vec<u8>,
    headers: HashMap<String, String>,
}

impl WebResponse {
    /// Create a default response 
    ///
    /// The code and status are defaulted to 200 "OK", which can be changed
    /// via the `set_code` method.  Headers and data are empty; see `set_data`
    /// and `set_header`.
    pub fn new() -> WebResponse {
        return WebResponse {
                code: 200,
                status: "OK".to_string(),
                data: Vec::new(),
                headers: HashMap::new(),
            };
    }

    /// Shortcut for creating a successful Unicode HTML response.
    pub fn new_html(body: String) -> WebResponse {
        let mut ret = WebResponse::new();   
        ret.set_data(body.into_bytes());
        ret.set_header("Content-Type", "text/html; charset=utf-8");
        return ret;
    }

    /// Set the HTTP status code and message
    pub fn set_code(&mut self, code: i32, status: &str) {
        self.code = code;
        self.status = status.to_string();
    }

    /// Set the response body
    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    /// Set a response header.  If it already exists, it will be overwritten.
    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name.to_string(), value.to_string());
    }
}


/// A request from a client
///
/// TODO: add the request body, or a way to fetch it.
///
pub struct WebRequest { 
    environ: HashMap<Vec<u8>, Vec<u8>>,
    path: String,
    method: String,
}

impl WebRequest {
    /// The CGI/WSGI like environment dictionary.
    ///
    /// Keys:
    ///
    /// * protocol = "http/1.0" or "http/1.1"
    /// * method = "get", "head", "options", ... 
    /// * path = "/full/path"
    /// * query_string = "k=v&k2=v2" or ""
    /// * http_xxx = "Header Value" 
    ///
    /// Note: protocol, method, and header names are lowercased,
    /// since they are defined to be case-insensitive.
    pub fn get_environ(&self) -> &HashMap<Vec<u8>, Vec<u8>> {
        return &self.environ;
    }

    /// The percent decoded and utf8 (lossy) decoded path.
    ///
    /// For the raw path, see environ[path].  
    /// Note: This does not normalize '/./' or  '/../' components.
    pub fn get_path(&self) -> &str {
        return self.path.as_slice();
    }

    /// The utf8 (lossy) decoded method, in lowercase.
    ///
    /// For the raw method, see environ[method].  
    pub fn get_method(&self) -> &str {
        return self.method.as_slice();
    }
}



/// The page handler function type 
pub type PageFunction = fn(&WebRequest) -> WebResponse;


struct DispatchRule {
    path: String,
    is_prefix: bool,
    methods: Vec<String>,
    page_fn: PageFunction
}

struct WorkerSharedContext {
    rules: Vec<DispatchRule>,
    acceptor: TcpAcceptor,
}

struct WorkerPrivateContext {
    shared_ctx: Arc<WorkerSharedContext>,
}


/// Processes HTTP requests
pub struct WebServer {
    nr_threads: i32,
    rules: Option<Vec<DispatchRule>>,
    thread_pool: ThreadPool,
    worker_shared_context: Option<Arc<WorkerSharedContext>>,
}

impl WebServer {
    pub fn new() -> WebServer {
        let ret = WebServer{
                nr_threads: 10,
                rules: Some(Vec::new()),
                thread_pool: ThreadPool::new(),
                worker_shared_context: None,
            };
        return ret;
    }

    /// Set number of worker threads.  Must be > 0.
    pub fn set_num_threads(&mut self, n: i32) {
        assert!(n > 0);
        self.nr_threads = n;
    }

    /// Add an exact match rule
    /// 
    /// methods: comma separated list of methods

    pub fn add_path(&mut self, methods: &str, path: &str, page_fn: PageFunction) {
        let fn_map = self.rules.as_mut().unwrap();
        let rule = DispatchRule { 
            path: path.to_string(), 
            is_prefix: false,
            page_fn: page_fn,
            methods: WebServer::parse_methods(methods),
        };
        fn_map.push(rule);
    }

    /// Add a prefix match rule
    /// 
    /// methods: comma separated list of methods
    pub fn add_path_prefix(&mut self, methods: &str, path: &str, page_fn: PageFunction) {
        let fn_map = self.rules.as_mut().unwrap();
        let rule = DispatchRule { 
            path: path.to_string(), 
            is_prefix: true,
            page_fn: page_fn,
            methods: WebServer::parse_methods(methods),
        };
        fn_map.push(rule);
    }

    /// Starts worker threads and enters supervisor loop.  If any worker
    /// threads fail, they will be respawned.  This function does not return.
    pub fn run(&mut self, address: &str, port: i32) {
        let addr = format!("{}:{}", address, port);
        println!("listening on {}", addr);
        let listener = TcpListener::bind(addr.as_slice());
        let acceptor = listener.listen().unwrap();
        
        // .clone doesn't work, compiler bug
        let page_fn_copy = self.rules.take().unwrap();

        let ctx = WorkerSharedContext {
            rules: page_fn_copy,
            acceptor: acceptor,
        };
        self.worker_shared_context = Some(Arc::new(ctx));

        println!("starting {} worker threads", self.nr_threads);
        for _ in range(0, self.nr_threads) {
            self.start_new_worker();
        }

        println!("starting monitor loop");
        loop {
            self.thread_pool.wait_for_thread_exit();
            println!("uh oh, a worker thread died");
            println!("starting another worker");
            self.start_new_worker();
        }
    }

    fn start_new_worker(&mut self) {
        let priv_ctx = WorkerPrivateContext {
            shared_ctx: self.worker_shared_context.as_mut().unwrap().clone(),
        };
        self.thread_pool.execute(move || {
            worker_thread_main(priv_ctx);
        });
    }

    fn parse_methods(methods: &str) -> Vec<String> {
        let mut parts = methods.split_str(",");
        let mut ret = Vec::new();
        for p in parts {
            ret.push(p.to_string());
        }
        return ret;
    }
}


fn worker_thread_main(ctx: WorkerPrivateContext) {
    let mut acceptor = ctx.shared_ctx.acceptor.clone();
    loop {
        let res = acceptor.accept();
        match res {
            Ok(sock) => process_http_connection(&ctx, sock),
            Err(err) => println!("socket error :-( {}", err)
        }
    }
}


// HTTP specific processing

#[allow(unused_parens)]
#[allow(unused_assignments)]
fn process_http_connection(ctx: &WorkerPrivateContext, stream: TcpStream) {
    let mut sentinel = HTTPContext { 
        stream: stream, 
        started_response: false 
    };

    // Read the request (headers only, not body yet)
    let req = read_request(&mut sentinel.stream);
    if req.is_none() {
        let mut resp = WebResponse::new();
        resp.set_code(400, "Bad Request");
        resp.set_data(b"Error 400: Bad Request".to_vec());
        sentinel.send_response(&resp);
        return;
    }

    let req = req.unwrap();
    println!("parsed request ok: path={}", req.path);

    // Do routing
    let ret = do_routing(ctx, &req);
    if let RoutingResult::FoundRule(page_fn) = ret {
        let response = (page_fn)(&req);
        sentinel.send_response(&response);
        return;
    }


/*
    println!("no rule matched {}", req.path);
    let mut response = WebResponse::new();
    if found_path_match {
        // TODO: return allow: header
        response.set_code(405, "Not Found, Bro");
        response.set_data(b"Error 405: Method not allowed".to_vec());
    } else {
        response.set_code(404, "Not Found, Bro");
        response.set_data(b"Error 404: Resource not found".to_vec());
    }
    sentinel.send_response(&response);
*/
}


enum RoutingResult {
    FoundRule(PageFunction),
    NoPathMatch,
    NoMethodMatch,
}

fn do_routing(ctx: &WorkerPrivateContext, req: &WebRequest) -> RoutingResult {
    let mut found_path_match = false;
    for rule in ctx.shared_ctx.rules.iter() {
        let mut matched;
        if rule.is_prefix {
            matched = req.path.as_slice().starts_with(rule.path.as_slice());
        } else {
            matched = rule.path == req.path;
        }
        if matched {
            found_path_match = true;
            // Now check methods
            for method in rule.methods.iter() {
                if *method == req.path {
                    // Found a rule match
                    return RoutingResult::FoundRule(rule.page_fn);
                }
            }
        }
    }
    if found_path_match {
        return RoutingResult::NoMethodMatch;
    } else {
        return RoutingResult::NoPathMatch;
    }
}


struct HTTPContext {
    stream: TcpStream,
    started_response: bool,
}

impl HTTPContext {
    fn send_response(&mut self, response: &WebResponse) {
        // todo: don't panic if logging fails?
        println!("sending response: code={}, body_length={}",
            response.code, response.data.len());

        let mut resp = String::new();
        resp.push_str(format!("HTTP/1.1 {} {}\r\n", 
            response.code, 
            response.status).as_slice());
        resp.push_str("Connection: close\r\n");
        resp.push_str(format!("Content-length: {}\r\n", 
                response.data.len()).as_slice());

        for (k, v) in response.headers.iter() {
            resp.push_str(k.as_slice());
            resp.push_str(": ");
            resp.push_str(v.as_slice());
            resp.push_str("\r\n");
        }

        resp.push_str("\r\n");

        // TODO: error check
        // We *don't* want to panic if we're already in a panic, and
        // sending the internal error message.
        self.started_response = true;
        let _ioret = self.stream.write_str(resp.as_slice());
        let _ioret = self.stream.write(response.data.as_slice());
    }
}

impl Drop for HTTPContext {
    /// If we paniced and/or are about to die, make sure client gets a 500
    fn drop(&mut self) {
        if !self.started_response {
            let mut resp = WebResponse::new();
            resp.set_code(500, "Uh oh :-(");
            resp.set_data(b"Error 500: Internal Error".to_vec());
            self.send_response(&resp);
        }
    }
}

fn read_request(stream: &mut TcpStream) -> Option<WebRequest> {
    // Read this amount at a time, if we want to set a max request size.
    let chunk_size = 4096;
    let mut req_buffer = Vec::<u8>::with_capacity(chunk_size);
    loop {
        let ioret = stream.push(chunk_size, &mut req_buffer);
        // todo: err handle
        let size = ioret.unwrap();
        //println!("read size {}", size);
        if size > 0 {
            //println!("req_buffer {}", req_buffer.len());
            let split_pos = byteutils::memmem(req_buffer.as_slice(), b"\r\n\r\n");
            if split_pos.is_some() {
                let split_pos = split_pos.unwrap();
                println!("read raw request: {} bytes", split_pos);
                let req = http_request::parse(req_buffer.as_slice());
                if req.is_ok() {
                    let req = req.ok().unwrap();
                    let ret = WebRequest {
                        environ: req.environ,
                        path: req.path,
                        method: req.method,
                    };
                    return Some(ret);
                } else {
                    return None;
                }
            }
        }
    }
}
