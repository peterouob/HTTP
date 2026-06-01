extern crate core;

pub mod error;

pub mod shutdown;
use shutdown::Shutdown;

pub mod connection;

mod parse;

mod router;
pub mod server;

use connection::Connection;
