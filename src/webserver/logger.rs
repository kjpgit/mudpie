use std::io;

pub struct Logger {
    logging_enabled: bool,
}

impl Logger {
    pub fn new(logging_enabled: bool) -> Logger {
        return Logger { 
            logging_enabled: logging_enabled,
        }
    }

    pub fn log_accept_error(&self, e: io::Error) {
        if self.logging_enabled {
            println!("Error from accept(): {}", e);
        }
    }

    pub fn log_read_request_error(&self, e: io::Error) {
        if self.logging_enabled {
            println!("Error when reading request: {}", e);
        }
    }

    // TODO: Better machine parsable output
    pub fn log_request_response(&self, method: &str, path: &str,
            code: i32, response_body_len: usize) {
        if ! self.logging_enabled { return; }
        println!("method={} path={} code={} body_len={}",
            method, path, code, response_body_len);
    }
}
