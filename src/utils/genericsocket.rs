use std::io;

/*
 * Trait for reading and writing to a socket.  
 * We technically don't need all the methods in Read and Write,
 * but it shouldn't be too onerous or non-standard to implement for an SSL
 * socket.
 *
 * We could trim this down to just expose the methods we need.
 * TODO: look at method name collisions and disambiguation.
 */

pub trait GenericSocket : io::Read + io::Write + Send { }

impl<T: io::Read + io::Write + Send> GenericSocket for T  { }
