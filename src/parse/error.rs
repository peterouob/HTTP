use crate::parse::parser::Status;
use std::fmt;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ParseError {
    HeaderName,
    HeaderValue,
    NewLine,
    StatusCode,
    Token,
    TooManyHeaders,
    Version,
    Reason,
    ReasonInvalidCode,
}

pub(crate) type ParseResult<T> = Result<Status<T>, ParseError>;

impl ParseError {
    #[inline]
    fn description_str(&self) -> &'static str {
        match *self {
            ParseError::HeaderName => "invalid header name",
            ParseError::HeaderValue => "invalid header value",
            ParseError::NewLine => "invalid new line",
            ParseError::StatusCode => "invalid response status",
            ParseError::Token => "invalid token",
            ParseError::TooManyHeaders => "too many headers",
            ParseError::Version => "invalid HTTP version",
            ParseError::Reason => "invalid reason",
            ParseError::ReasonInvalidCode => "invalid reason code",
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.description_str())
    }
}

impl std::error::Error for ParseError {
    fn description(&self) -> &str {
        self.description_str()
    }
}
