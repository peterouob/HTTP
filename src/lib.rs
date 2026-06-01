extern crate core;

pub mod error;

pub mod shutdown;
use shutdown::Shutdown;

pub mod connection;

mod parse;

pub mod server;
mod router;

use connection::Connection;
