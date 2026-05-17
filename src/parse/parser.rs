use crate::expect;
use crate::next;
use crate::parse::error::{ParseError, ParseResult};
use crate::parse::http_method::Method;
use crate::parse::iter::ParseBuffer;
use std::fmt;
use std::fmt::{Formatter, write};

/*
 *  ----------------------------
 *  start line
 *  ----------------------------
 *  [method] [path] [Protocol version]
 *  GET        /    HTTP/1.1
 *  ----------------------------
 *  header [field name]: [field value]
 *  ----------------------------
 *  Host: www.google.com
 *  Accept: text/html
 *  ----------------------------
 *  CRLF
 *  ----------------------------
 *  body
 *  ----------------------------
 * */

#[derive(Debug, Eq, PartialEq)]
pub struct Header<'buf> {
    name: &'buf str,
    value: &'buf [u8],
}

#[derive(Debug, Eq, PartialEq)]
pub struct Request<'h, 'b> {
    pub method: Option<Method<'b>>,
    pub path: Option<&'b str>,
    pub version: Option<u8>,
    pub headers: &'h mut [Header<'b>],
}

impl<'h, 'b> fmt::Display for Request<'h, 'b> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write(
            f,
            format_args!(
                "method:{:?}, path:{:?}, version:{:?}, headers:{:?}",
                self.method, self.path, self.version, self.headers
            ),
        )
    }
}

impl<'h, 'b> Request<'h, 'b> {
    #[inline]
    pub fn new(headers: &'h mut [Header<'b>]) -> Request<'h, 'b> {
        Request {
            method: None,
            path: None,
            version: None,
            headers,
        }
    }
}

#[derive(Debug)]
pub struct Response<'h, 'b> {
    pub version: Option<u16>,
    pub status_code: Option<u8>,
    pub msg: Option<&'b str>,
    pub headers: &'h mut [Header<'b>],
}

impl<'h, 'b> fmt::Display for Response<'h, 'b> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write(
            f,
            format_args!(
                "version:{:?}, status_code:{:?}, msg:{:?}, headers:{:?}",
                self.version, self.status_code, self.msg, self.headers
            ),
        )
    }
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Debug)]
pub enum Status<T> {
    Partial,
    Complete(T),
}

impl<T> Status<T> {
    #[inline]
    pub fn is_complete(&self) -> bool {
        match *self {
            Status::Complete(..) => true,
            Status::Partial => false,
        }
    }

    #[inline]
    pub fn is_partial(&self) -> bool {
        match *self {
            Status::Partial => true,
            Status::Complete(..) => false,
        }
    }

    #[inline]
    pub fn unwrap(self) -> T {
        match self {
            Status::Complete(t) => t,
            Status::Partial => panic!("Tried to unwrap Status::Partial"),
        }
    }
}

#[inline]
fn skip_empty_line(bytes: &mut ParseBuffer) -> ParseResult<()> {
    loop {
        let b = bytes.peek();
        match b {
            Some(b'\r') => {
                bytes.advance(1);
                expect!(bytes.peek()== b'\n' => Err(ParseError::NewLine));
            }
            Some(b'\n') => {
                bytes.advance(1);
            }
            Some(..) => {
                bytes.slice();
                return Ok(Status::Complete(()));
            }
            None => return Ok(Status::Partial),
        }
    }
}

#[inline]
fn skip_space_line(bytes: &mut ParseBuffer) -> ParseResult<()> {
    loop {
        let b = bytes.peek();
        match b {
            Some(b' ') => {
                bytes.advance(1);
            }
            Some(..) => {
                bytes.slice();
                return Ok(Status::Complete(()));
            }
            None => return Ok(Status::Partial),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_skip_empty_line() {
        let mut buf = ParseBuffer::new(b"Hello");
        assert_eq!(skip_empty_line(&mut buf).unwrap(), Status::Complete(()));
        assert_eq!(buf.cursor, 0);

        let mut buf = ParseBuffer::new(b"\r\nHello");
        assert_eq!(skip_empty_line(&mut buf).unwrap(), Status::Complete(()));
        assert_eq!(buf.cursor, 2);

        let mut buf = ParseBuffer::new(b"\nHello");
        assert_eq!(skip_empty_line(&mut buf).unwrap(), Status::Complete(()));
        assert_eq!(buf.cursor, 1);

        let mut buf = ParseBuffer::new(b"");
        assert_eq!(skip_empty_line(&mut buf).unwrap(), Status::Partial);

        let mut buf = ParseBuffer::new(b"\r\n");
        assert_eq!(skip_empty_line(&mut buf).unwrap(), Status::Partial);

        let mut buf = ParseBuffer::new(b"\rHello");
        assert!(skip_empty_line(&mut buf).is_err());
    }

    #[test]
    fn test_skip_space_line() {
        let mut buf = ParseBuffer::new(b" Hello");
        assert_eq!(skip_space_line(&mut buf).unwrap(), Status::Complete(()));

        let mut buf = ParseBuffer::new(b" ");
        assert_eq!(skip_space_line(&mut buf).unwrap(), Status::Partial);
    }
}
