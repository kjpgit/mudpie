#![feature(collections)]
#![feature(io)]
#![feature(tcp)]
#![feature(std_misc)]

pub use webserver::{WebServer, WebRequest, WebResponse};
pub use webserver::{PageFunction};
pub use utils::escape::html_element_escape;
mod utils;
mod webserver;
