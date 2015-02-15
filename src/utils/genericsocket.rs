use std::io;

/*
 * Trait for reading and writing to a socket.  
 * We technically don't need all the methods in Read and Write,
 * but it shouldn't be too onerous or non-standard to implement for an SSL
 * socket.
 *
 * If we needed to, we could make this a custom trait and provide a default
 * generic implementation for types implementing Read+Write.
 * e.g. impl<T: io::Read + io::Write> GenericSocket for T 
 */

pub trait GenericSocket : io::Read + io::Write + Send { }

impl<T: io::Read + io::Write + Send> GenericSocket for T  { }
