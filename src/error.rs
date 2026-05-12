use thiserror::Error;
#[derive(Error, Debug)]
pub enum TCPSocketError {
    #[error("socket config error")]
    SocketConfig(String),
}
