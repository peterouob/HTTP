#[derive(Debug, PartialEq, Eq)]
pub enum Method<'buf> {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    PATCH,
    CONNECT,
    TRACE,
    EXTENSION(&'buf [u8]),
}
