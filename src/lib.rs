#![feature(vec_push_all)]
#![feature(owned_ascii_ext)]
#![feature(vec_resize)]
#![feature(tcp)]

pub use webserver::{WebServer, WebRequest, WebResponse};
pub use webserver::{PageFunction};
pub use utils::escape::html_element_escape;
mod utils;
mod webserver;
