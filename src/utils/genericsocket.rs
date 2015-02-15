use std::io;

/*
 * Trait for reading and writing to a socket.  
 * Designed to be easily wrappable by SSL.
 */

pub trait GenericSocket : Send {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error>;
    fn write_all(&mut self, buf: &[u8]) -> Result<(), io::Error>;
}

impl<T: io::Read + io::Write + Send> GenericSocket for T {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        return (self as &mut io::Read).read(buf);
    }
    fn write_all(&mut self, buf: &[u8]) -> Result<(), io::Error> {
        return (self as &mut io::Write).write_all(buf);
    }
}


