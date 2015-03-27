//! Helper module for reading a WebRequest

use std;
use std::io;
use std::ascii::OwnedAsciiExt; 

use super::WebRequest;
use utils::genericsocket::GenericSocket;
use utils;


// Possible errors from `read_request`
pub enum Error {
    IoError(io::Error),
    InvalidRequest,
    InvalidVersion,
    LengthRequired,
    TooLarge,
}

// Auto convert io::IOError into our module specific error
impl std::error::FromError<io::Error> for Error {
    fn from_error(err: io::Error) -> Error {
        Error::IoError(err)
    }
}


// TODO: split the body reading out
// Read a full request from the client (headers and body)
// max_size: max body size
//
// We need a Reader+Writer, due to stupid HTTP 100-continue.
// We transparently send the 100-Continue if expected of us.  However, the more
// educated thing to do, for apps that actually care about this, would be to
// call the app code first and let it validate the headers.
pub fn read_request(stream: &mut GenericSocket, max_size: usize) 
        -> Result<WebRequest, Error> {
    let mut req_buffer = Vec::<u8>::with_capacity(4096);
    let req_size = try!(read_until_headers_end(&mut req_buffer, stream));

    // Try to parse it
    let req = match utils::http_request::parse(&req_buffer[..req_size]) {
        Err(utils::http_request::ParseError::BadVersion) => 
            return Err(Error::InvalidVersion),
        Err(..) => return Err(Error::InvalidRequest),
        Ok(parsed_req) => parsed_req,
    };

    // See if there's a body to read too.  
    let mut body = Vec::new();

    // We don't currently support chunked
    if req.environ.contains_key(&b"http_transfer-encoding"[..]) {
        return Err(Error::LengthRequired);
    }

    { // borrow scope for req.environ
    let clen = req.environ.get(&b"http_content-length"[..]);
    if clen.is_some() {
        let clen = match utils::byteutils::parse_u64(&clen.unwrap()) {
            // unparseable content-length
            None => return Err(Error::InvalidRequest),
            Some(clen) => clen,
        };

        if clen > max_size as u64 {
            return Err(Error::TooLarge);
        }

        // Cast it down, as we read in memory
        let clen = clen as usize;

        // Send 100-continue if needed
        if needs_100_continue(&req) {
            let cont = b"HTTP/1.1 100 Continue\r\n\r\n";
            try!(stream.write_all(cont));
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

        body = body_buffer;
    }
    }

    // All done
    let ret = WebRequest {
        environ: req.environ,
        path: req.path,
        method: req.method,
        body: body,
    };
    return Ok(ret);
}


fn needs_100_continue(req: &utils::http_request::Request) -> bool {
    let val = req.environ.get(&b"http_expect"[..]);
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
        stream: &mut GenericSocket) -> Result<usize, io::Error> 
{
    // Craptastic new io copying; with_extra isn't supported yet
    // and is unsafe.
    let chunk_size = 4096;
    let mut chunk_buff = Vec::with_capacity(chunk_size);
    chunk_buff.resize(chunk_size, 0);

    loop { 
        // Try to read some more data
        let size = try!(stream.read(&mut chunk_buff));
        if size == 0 {
            return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "connection closed while reading request headers", 
                    None));
        }
        buffer.push_all(&chunk_buff[0..size]);

        let split_pos = utils::byteutils::memmem(&buffer, b"\r\n\r\n");
        if split_pos.is_none() {
            continue;
        }
        return Ok(split_pos.unwrap() + 4);
    }
}


// Read until the buffer is at least size bytes long
// Note: extra data may be in the buffer.
fn read_until_size(buffer: &mut Vec<u8>,
        stream: &mut GenericSocket, size: usize) -> Result<(), io::Error>
{
    let chunk_size = 4096;
    let mut chunk_buff = Vec::with_capacity(chunk_size);
    chunk_buff.resize(chunk_size, 0);

    while buffer.len() < size {
        let size = try!(stream.read(&mut chunk_buff));
        if size == 0 {
            return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "connection closed while reading request body", 
                    None));
        }
        buffer.push_all(&chunk_buff[0..size]);
    }
    return Ok(());
}
