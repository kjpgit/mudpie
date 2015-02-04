use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::old_io::{TcpListener, TcpStream};
use std::old_io::net::tcp::TcpAcceptor;
use std::old_io::{Acceptor, Listener};
use std::ascii::OwnedAsciiExt;

use utils::threadpool::ThreadPool;
use self::write_response::write_response;

mod read_request;
mod write_response;

static DEFAULT_MAX_REQUEST_BODY_SIZE: usize = 1_000_000;


/// A response that will be sent to the client (code, headers, body)
pub struct WebResponse {
    code: i32,
    status: String, 
    body: Vec<u8>,
    headers: HashMap<String, String>,
}

impl WebResponse {
    /// Create a default response 
    ///
    /// The code and status are defaulted to 200 "OK", which can be changed
    /// via the `set_code` method.  Headers and body are empty; see `set_body`
    /// and `set_header`.
    pub fn new() -> WebResponse {
        return WebResponse {
                code: 200,
                status: "OK".to_string(),
                body: Vec::new(),
                headers: HashMap::new(),
            };
    }

    /// Shortcut for creating a successful Unicode HTML response.
    ///
    /// This is equivalent to: 
    ///
    /// ```ignore
    /// set_body_str(body)
    /// set_header("Content-Type", "text/html; charset=utf-8");
    /// ```
    pub fn new_html(body: String) -> WebResponse {
        let mut ret = WebResponse::new();   
        ret.set_body(body.into_bytes());
        ret.set_header("Content-Type", "text/html; charset=utf-8");
        return ret;
    }

    /// Set the HTTP status code and message.  The message should contain ASCII
    /// characters only.
    pub fn set_code(&mut self, code: i32, status: &str) {
        self.code = code;
        self.status = status.to_string();
    }

    /// Set the response body
    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = body;
    }

    /// Set the response body as the UTF-8 encoded bytes from `body`.
    /// Equivalent to set_body(body.as_bytes())
    pub fn set_body_str(&mut self, body: &str) {
        self.body = body.as_bytes().to_vec();
    }

    /// Set a response header.  If it already exists, it will be overwritten.
    /// Header names and values should use ASCII characters only.
    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers.insert(name.to_string(), value.to_string());
    }
}


/// A request from a client
///
pub struct WebRequest { 
    environ: HashMap<Vec<u8>, Vec<u8>>,
    path: String,
    method: String,
    body: Vec<u8>,
}

impl WebRequest {
    /// The CGI/WSGI like environment dictionary.
    ///
    /// Keys:
    ///
    /// * protocol = "http/1.0" or "http/1.1"
    /// * method = "get", "head", "options", ... 
    /// * path = "/full/path"
    /// * query_string = "k=v&k2=v2" or "" (empty)
    /// * http_xxx = "Header Value".  Example: http_user-agent = "Mozilla Firefox"
    ///
    /// Note: protocol, method, and header names are lowercased,
    /// since they are defined to be case-insensitive.
    /// 
    /// If the same header name was repeated in the request, the values will be
    /// concatenated, in order received, separated by a comma.
    pub fn get_environ(&self) -> &HashMap<Vec<u8>, Vec<u8>> {
        return &self.environ;
    }

    /// The percent decoded and utf8 (lossy) decoded path.
    ///
    /// For the raw path, see environ[path].  
    /// Note: This does not normalize '/./' or  '/../' components.
    pub fn get_path(&self) -> &str {
        return &*self.path;
    }

    /// The utf8 (lossy) decoded method, in lowercase.
    ///
    /// For the raw method, see environ[method].  
    pub fn get_method(&self) -> &str {
        return &*self.method;
    }

    /// The request body.  Note that HTTP requests do not distinguish a null vs
    /// 0 length body, so this no longer returns an Option.
    pub fn get_body(&self) -> &[u8] {
        return &*self.body;
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
    max_request_body_size: usize,
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
    max_request_body_size: usize,
}

impl WebServer {
    pub fn new() -> WebServer {
        let ret = WebServer{
                nr_threads: 10,
                rules: Some(Vec::new()),
                thread_pool: ThreadPool::new(),
                worker_shared_context: None,
                max_request_body_size: DEFAULT_MAX_REQUEST_BODY_SIZE,
            };
        return ret;
    }

    /// Set number of worker threads.  Must be > 0.
    pub fn set_num_threads(&mut self, n: i32) {
        assert!(n > 0);
        self.nr_threads = n;
    }

    /// Set the maximum request body size.  Larger requests will generate 
    /// a 413 error.
    pub fn set_max_request_body_size(&mut self, size: usize) {
        self.max_request_body_size = size;
    }

    /// Add an exact match rule
    /// 
    /// methods: comma separated list of methods

    pub fn add_path(&mut self, methods: &str, path: &str, 
            page_fn: PageFunction) {
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
    pub fn add_path_prefix(&mut self, methods: &str, path: &str, 
            page_fn: PageFunction) {
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
        let listener = TcpListener::bind(&*addr);
        let acceptor = listener.listen().unwrap();
        
        // .clone doesn't work, compiler bug
        let page_fn_copy = self.rules.take().unwrap();

        let ctx = WorkerSharedContext {
            rules: page_fn_copy,
            acceptor: acceptor,
            max_request_body_size: self.max_request_body_size,
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

    // returns methods in lowercase
    fn parse_methods(methods: &str) -> Vec<String> {
        let mut parts = methods.split_str(",");
        let mut ret = Vec::new();
        for p in parts {
            let method = p.trim().to_string().into_ascii_lowercase();
            ret.push(method);
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


// HTTP specific socket processing
fn process_http_connection(ctx: &WorkerPrivateContext, stream: TcpStream) {
    let mut stream = stream;

    // Read full request (headers and body)
    let req = match read_request::read_request(&mut stream,
            ctx.shared_ctx.max_request_body_size) {
        Err(read_request::Error::InvalidRequest) => {
            let mut resp = WebResponse::new();
            resp.set_code(400, "Bad Request");
            resp.set_body_str("Error 400: Bad Request");
            write_response(&mut stream, None, &resp);
            return;
        },
        Err(read_request::Error::LengthRequired) => {
            let mut resp = WebResponse::new();
            resp.set_code(411, "Length Required");
            resp.set_body_str("Error 411: Length Required");
            write_response(&mut stream, None, &resp);
            return;
        },
        Err(read_request::Error::InvalidVersion) => {
            let mut resp = WebResponse::new();
            resp.set_code(505, "Version not Supported");
            resp.set_body_str("Error 505: Version not Supported");
            write_response(&mut stream, None, &resp);
            return;
        },
        Err(read_request::Error::TooLarge) => {
            let mut resp = WebResponse::new();
            resp.set_code(413, "Request Entity Too Large");
            resp.set_body_str("Error 413: Request Entity Too Large");
            write_response(&mut stream, None, &resp);
            return;
        },
        Err(read_request::Error::IoError(e)) => {
            println!("IoError during request: {}", e);
            return;
        },
        Ok(req) => req,
    };

    // Do routing
    let ret = do_routing(ctx, &req);
    let page_fn = match ret {
        RoutingResult::FoundRule(page_fn) => page_fn,
        RoutingResult::NoPathMatch => {
            let mut resp = WebResponse::new();
            resp.set_code(404, "Not Found");
            resp.set_body_str("Error 404: Resource not found");
            write_response(&mut stream, Some(&req), &resp);
            return;
        }
        RoutingResult::NoMethodMatch(methods) => {
            let mut resp = WebResponse::new();
            resp.set_code(405, "Method not allowed");
            resp.set_body_str("Error 405: Method not allowed");
            let methods_joined = methods.connect(", ");
            resp.set_header("Allow", &*methods_joined);
            write_response(&mut stream, Some(&req), &resp);
            return;
        }
    };


    // Run the handler.  If it panics, the sentinel will send a 500.
    let mut sentinel = HTTPConnectionSentinel { 
        request: req,
        stream: stream, 
        armed: true 
    };
    let response = (page_fn)(&sentinel.request);
    sentinel.armed = false;
    write_response(&mut sentinel.stream, 
        Some(&sentinel.request),
        &response);
}


// A sentinel that sends a 500 error unless armed=false
struct HTTPConnectionSentinel {
    stream: TcpStream,
    armed: bool,
    request: WebRequest,
}

impl Drop for HTTPConnectionSentinel {
    /// If we paniced and/or are about to die, make sure client gets a 500
    fn drop(&mut self) {
        if self.armed {
            let mut resp = WebResponse::new();
            resp.set_code(500, "Uh oh :-(");
            resp.set_body_str("Error 500: Internal error in handler function");
            write_response(&mut self.stream, Some(&self.request), &resp);
        }
    }
}


enum RoutingResult {
    FoundRule(PageFunction),
    NoPathMatch,
    NoMethodMatch(Vec<String>),
}

fn do_routing(ctx: &WorkerPrivateContext, req: &WebRequest) -> RoutingResult {
    let mut found_path_match = false;
    let mut found_methods = HashSet::<&str>::new();
    for rule in ctx.shared_ctx.rules.iter() {
        let mut matched;
        if rule.is_prefix {
            matched = req.path.starts_with(&*rule.path);
        } else {
            matched = req.path == rule.path;
        }
        if matched {
            found_path_match = true;
            // Now check methods
            for method in rule.methods.iter() {
                found_methods.insert(&**method);
                if *method == req.method {
                    // Found a rule match
                    return RoutingResult::FoundRule(rule.page_fn);
                }
            }
        }
    }
    if found_path_match {
        let mut methods = Vec::new();
        for method in found_methods.iter() {
            methods.push(method.to_string());
        }
        return RoutingResult::NoMethodMatch(methods);
    } else {
        return RoutingResult::NoPathMatch;
    }
}
