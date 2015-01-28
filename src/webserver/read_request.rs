//! Helper module for reading a WebRequest

use std;
use std::io::Reader;
use std::ascii::OwnedAsciiExt; 

use super::WebRequest;
use utils;


// Possible errors from `read_request`
pub enum Error {
    IoError(std::io::IoError),
    InvalidRequest,
    TooLarge,
}

// Auto convert io::IOError into our module specific error
impl std::error::FromError<std::io::IoError> for Error {
    fn from_error(err: std::io::IoError) -> Error {
        Error::IoError(err)
    }
}


// Read a full request from the client (headers and body)
// max_size: max body size
//
// We need a Reader+Writer, due to stupid HTTP 100-continue.
// We transparently send the 100-Continue if expected of us.  However, the more
// educated thing to do, for apps that actually care about this, would be to
// call the app code first and let it validate the headers.
pub fn read_request<T: Reader+Writer>(stream: &mut T, max_size: u64) 
        -> Result<WebRequest, Error> {
    let mut req_buffer = Vec::<u8>::with_capacity(4096);
    let req_size = try!(read_until_headers_end(&mut req_buffer, stream));
    println!("read raw request: {} bytes", req_size);

    // Try to parse it
    let req = match utils::http_request::parse(&req_buffer[..req_size]) {
        Err(..) => return Err(Error::InvalidRequest),
        Ok(parsed_req) => parsed_req,
    };

    // Valid request.  
    println!("parsed request ok: method={}, path={}", req.method, req.path);

    // See if there's a body to read too.  
    let mut body = None;
    {
        // borrowing req.environ here
        let clen = req.environ.get(b"http_content-length");
        if clen.is_some() {
            let clen = match utils::byteutils::parse_u64(&**clen.unwrap()) {
                // unparseable content-length
                None => return Err(Error::InvalidRequest),
                Some(clen) => clen,
            };

            println!("body size: {} bytes", clen);

            if clen > max_size {
                return Err(Error::TooLarge);
            }

            // Cast it down, as we read in memory
            assert!(clen < std::usize::MAX as u64);
            let clen = clen as usize;

            // Send 100-continue if needed
            if needs_100_continue(&req) {
                println!("sending 100 continue");
                let cont = b"HTTP/1.1 100 Continue\r\n\r\n";
                try!(stream.write(cont));
            }

            // Start one new buffer, so we don't copy when done
            let mut body_buffer = req_buffer[req_size..].to_vec();
            // Can free some memory
            drop(req_buffer);

            // Read the body
            try!(read_until_size(&mut body_buffer, stream, clen));
            assert!(body_buffer.len() >= clen);

            // Make sure not to include an extra pipelined request
            body_buffer.truncate(clen);
            assert!(body_buffer.len() == clen);

            body = Some(body_buffer);
        }
    }

    // Got headers and body ok
    let ret = WebRequest {
        environ: req.environ,
        path: req.path,
        method: req.method,
        body: body,
    };
    return Ok(ret);
}


fn needs_100_continue(req: &utils::http_request::Request) -> bool {
    let val = req.environ.get(b"http_expect");
    if val.is_none() {
        return false;
    }
    let val = val.unwrap().clone().into_ascii_lowercase();
    if val == b"100-continue" {
        return true;
    } else {
        // Note: the RFC only defines 100-continue, and says we MAY
        // generate 417 (Expectation Failed) here.
        return false;
    }
}


// Read until \r\n\r\n, which terminates the request headers
// Note: extra data may be in the buffer.
fn read_until_headers_end(buffer: &mut Vec<u8>,
        stream: &mut Reader) -> Result<usize, std::io::IoError> 
{
    let chunk_size = 4096;
    loop { 
        // Try to read some more data
        let size = try!(stream.push(chunk_size, buffer));
        //println!("read size {}", size);
        if size == 0 {
            continue;
        }

        //println!("req_buffer {}", req_buffer.len());
        let split_pos = utils::byteutils::memmem(&**buffer, b"\r\n\r\n");
        if split_pos.is_none() {
            continue;
        }
        return Ok(split_pos.unwrap() + 4);
    }
}


// Read until the buffer is at least size bytes long
// Note: extra data may be in the buffer.
fn read_until_size(buffer: &mut Vec<u8>,
        stream: &mut Reader, size: usize) -> Result<(), std::io::IoError>
{
    let chunk_size = 4096;
    while buffer.len() < size {
        try!(stream.push(chunk_size, buffer));
    }
    return Ok(());
}
