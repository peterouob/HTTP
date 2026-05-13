use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("incomplete input, need more bytes")]
    Incomplete,

    #[error("request line too long: {0} bytes (max {1})")]
    RequestLineTooLong(usize, usize),

    #[error("invalid HTTP method: {0:?}")]
    InvalidMethod(String),

    #[error("request target is empty")]
    EmptyRequestTarget,

    #[error("request target contains invalid character at byte {0}: {1:#04x}")]
    InvalidRequestTarget(usize, u8),

    #[error("unsupported HTTP version: {0:?}")]
    UnsupportedHttpVersion(String),

    #[error("malformed request line: missing SP between {0} and {1}")]
    MalformedRequestLine(&'static str, &'static str),

    #[error("header section too large: {0} bytes (max {1})")]
    HeaderSectionTooLarge(usize, usize),

    #[error("too many headers: {0} (max {1})")]
    TooManyHeaders(usize, usize),

    #[error("header name is empty")]
    EmptyHeaderName,

    #[error("header name contains invalid character at position {0}: {1:#04x}")]
    InvalidHeaderName(usize, u8),

    #[error("header value for {0:?} contains invalid byte at position {1}: {2:#04x}")]
    InvalidHeaderValue(String, usize, u8),

    #[error("header line too long: {0} bytes (max {1})")]
    HeaderLineTooLong(usize, usize),

    #[error("header missing colon separator: {0:?}")]
    MissingHeaderColon(String),

    #[error("obsolete line folding is not supported (RFC 7230 §3.2.6)")]
    ObsoleteFolding,

    #[error("Content-Length value is not a valid integer: {0:?}")]
    InvalidContentLength(String),

    #[error("conflicting Content-Length headers: {0} vs {1}")]
    ConflictingContentLength(u64, u64),

    #[error("invalid Transfer-Encoding value: {0:?}")]
    InvalidTransferEncoding(String),

    #[error("both Content-Length and Transfer-Encoding: chunked are present")]
    ContentLengthWithChunked,

    #[error("missing Host header (required by RFC 7230 §5.4)")]
    MissingHost,

    #[error("multiple Host headers found")]
    MultipleHostHeaders,

    #[error("invalid Host value: {0:?}")]
    InvalidHost(String),

    #[error("expected CRLF but found {0:#04x} {1:#04x}")]
    InvalidLineEnding(u8, u8),

    #[error("unexpected end of stream after {0} bytes")]
    UnexpectedEof(usize),
}
