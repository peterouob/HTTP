use crate::next;
use std::collections::HashMap;
use crate::{complete, expect, newline};
use crate::parse::error::{ParseError, ParseResult};
use crate::parse::iter::ParseBuffer;
use crate::parse::parse_utils::{skip_empty_line, parse_method, parse_uri, parse_version, match_header_name_vector, match_header_value};
use std::fmt;
use std::fmt::{Formatter, write};
use crate::parse::tchar::{is_header_name_token, is_header_value};
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

#[derive(Debug,Eq,PartialEq)]
pub struct HeaderMap<'buf> {
    header: HashMap<&'buf str, &'buf [u8]>,
}

impl<'buf> HeaderMap<'buf> {
    pub fn new() -> Self { HeaderMap { header: HashMap::new() } }

    pub fn insert(&mut self, key: &'buf str, value: &'buf [u8]) {
        self.header.insert(key, value);
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Request<'h, 'b> {
    pub method: Option<&'b str>,
    pub path: Option<&'b str>,
    pub version: Option<u8>,
    pub headers: &'h mut HeaderMap<'b>,
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
    pub fn new(headers: &'h mut HeaderMap<'b>) -> Request<'h, 'b> {
        Request {
            method: None,
            path: None,
            version: None,
            headers,
        }
    }

    pub fn parse_header(&mut self, bytes: &'b [u8])-> Result<Status<()>, ParseError> {
        let mut bytes = ParseBuffer::new(bytes);
        complete!(skip_empty_line(&mut bytes));
        let method = complete!(parse_method(&mut bytes));
        self.method = Some(method);
        self.path = Some(complete!(parse_uri(&mut bytes)));
        self.version = Some(complete!(parse_version(&mut bytes)));

        newline!(bytes);

        parse_header_iter(&mut bytes, self.headers).expect("Failed to parse headers");

        Ok(Status::Complete(()))
    }
}

pub fn parse_header_iter<'a>(bytes: &mut ParseBuffer<'a>,headers: &mut HeaderMap<'a>) -> ParseResult<usize> {
    let start = bytes.as_ref().as_ptr() as usize;
    let mut result = Err(ParseError::TooManyHeaders);

    'headers: loop {
        let b = next!(bytes);

        if b == b'\r' {
            expect!(bytes.peek()==b'\n'=>Err(ParseError::NewLine));
            let end = bytes.as_ref().as_ptr() as usize;
            result = Ok(Status::Complete(end-start));
            break
        }

        if b == b'\n' {
            let end = bytes.as_ref().as_ptr() as usize;
            result = Ok(Status::Complete(end-start));
            break
        }

        if !is_header_name_token(b) {
            result = Err(ParseError::HeaderName);
            break
        }

        let header_name: &str = 'name: loop {
            match_header_name_vector(bytes);
            let mut b = next!(bytes);

            let bslice = bytes.sub_slice(1);

            // TODO: avoid unwrap to get value
            let name = str::from_utf8(bslice.unwrap()).unwrap();

            if b == b':' {
                break 'name name
            }

        };

        let mut b;

        let header_value_slice = 'value: loop {

            'whitespace_after_colon: loop {
                b = next!(bytes);

                if b == b' ' || b == b'\t' {
                    bytes.slice();
                    continue 'whitespace_after_colon;
                }

                if is_header_value(b) {
                    break 'whitespace_after_colon;
                }

                if b == b'\r' {
                    expect!(bytes.peek()==b'\n'=>Err(ParseError::NewLine));
                }else if b != b'\n' {
                   return  Err(ParseError::HeaderValue);
                }

                let whitespace_slice = bytes.slice().unwrap();
                println!("whitespace_slice: {:?}", whitespace_slice);
                break 'value &whitespace_slice[0..0];
            }

             loop {
                match_header_value(bytes);
                let b = next!(bytes);

                let skip_num = if b == b'\r' {
                    expect!(bytes.peek()==b'\n'=>Err(ParseError::NewLine));
                    2
                }else if b == b'\n' {
                    1
                }else {
                    return Err(ParseError::HeaderValue);
                };

               break 'value bytes.sub_slice(skip_num).unwrap();
            }
        };


        let header_value = if let Some(last_visible) = header_value_slice
            .iter()
            .rposition(|b| *b != b' ' && *b != b'\t' && *b != b'\r' && *b != b'\n')
        {
            &header_value_slice[..last_visible+1]
        }else {
            header_value_slice
        };

        headers.insert(header_name, header_value);
    }
    result
}

#[derive(Debug)]
pub struct Response<'h, 'b> {
    pub version: Option<u16>,
    pub status_code: Option<u8>,
    pub msg: Option<&'b str>,
    pub headers: &'h mut Vec<HeaderMap<'b>>,
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

#[cfg(test)]
mod test {
    use crate::parse::parser::Status::Complete;
    use super::*;

    #[test]
    fn test_request_parse() {
        let mut headers = HeaderMap::new();
        let mut req = Request::new(&mut headers);
        let url = "GET /index.html HTTP/1.1\r\nHost: ://example.com\r\n\r\n";

        let _result = req.parse_header(url.as_ref());

        assert_eq!(req.method, Some("GET"));
        assert_eq!(req.path, Some("/index.html"));
        assert_eq!(req.version, Some(1));
        assert_eq!(req.headers.header.get("Host"), Some(&b"://example.com"[..]).as_ref());
    }

    #[test]
    fn test_request_parse_with_multiple_headers() {
        let mut headers = HeaderMap::new();
        let mut req = Request::new(&mut headers);
        let url = "GET /index.html HTTP/1.1\r\nHost: ://example.com\r\nUser-Agent: Mozilla/5.0\r\nAccept: text/html\r\nConnection: close\r\n\r\n";

        let _result = req.parse_header(url.as_ref());
        assert_eq!(req.method, Some("GET"));
        assert_eq!(req.path, Some("/index.html"));
        assert_eq!(req.version, Some(1));
        assert_eq!(req.headers.header.get("Host"), Some(&b"://example.com"[..]).as_ref());
        assert_eq!(req.headers.header.get("User-Agent"), Some(&b"Mozilla/5.0"[..]).as_ref());
        assert_eq!(req.headers.header.get("Accept"), Some(&b"text/html"[..]).as_ref());
        assert_eq!(req.headers.header.get("Connection"), Some(&b"close"[..]).as_ref());
    }

    fn parse(input: &str) -> Result<Status<()>, ParseError> {
        let mut headers = HeaderMap::new();
        let mut req = Request::new(&mut headers);
        let result = req.parse_header(input.as_bytes());
        result
    }


    #[test]
    fn test_empty_buffer() {
        let result = parse("");
        assert_eq!(result, Ok(Status::Partial));
    }

    #[test]
    fn test_only_method() {
        let result = parse("GET ");
        assert_eq!(result, Err(ParseError::Token));
    }

    #[test]
    fn test_method_and_uri_no_version() {
        let result = parse("GET /index.html ");
        assert_eq!(result, Ok(Status::Partial));
    }

    #[test]
    fn test_no_final_crlf() {
        let result = parse("GET /index.html HTTP/1.1");
        assert_eq!(result, Ok(Status::Partial));
    }

    #[test]
    fn test_invalid_method_char() {
        let result = parse("G@T /index.html HTTP/1.1\r\n");
        assert!(matches!(result, Err(ParseError::Token)));
    }

    #[test]
    fn test_empty_method() {
        let result = parse(" /index.html HTTP/1.1\r\n");
        assert!(matches!(result, Err(_)));
    }

    #[test]
    fn test_empty_uri() {
        let result = parse("GET  HTTP/1.1\r\n");
        assert!(matches!(result, Err(ParseError::Token)));
    }

    #[test]
    fn test_uri_with_ctl_char() {
        let result = parse("GET /pa\x01th HTTP/1.1\r\n");
        assert!(matches!(result, Err(ParseError::Token)));
    }

    #[test]
    fn test_invalid_version() {
        let result = parse("GET /index.html HTTP/2.0\r\n");
        assert!(matches!(result, Err(ParseError::Version)));
    }

    #[test]
    fn test_lowercase_http() {
        let result = parse("GET /index.html http/1.1\r\n");
        assert!(matches!(result, Err(ParseError::Version)));
    }

    #[test]
    fn test_leading_crlf() {
        let mut headers = HeaderMap::new();
        let mut req = Request::new(&mut headers);
        let result = req.parse_header(b"\r\nGET /index.html HTTP/1.1\r\n");
        assert_eq!(result, Ok(Complete(())));
        assert_eq!(req.method, Some("GET"));
    }

    #[test]
    fn test_leading_lf() {
        let mut headers = HeaderMap::new();
        let mut req = Request::new(&mut headers);
        let result = req.parse_header(b"\nGET /index.html HTTP/1.1\r\n");
        assert_eq!(result, Ok(Complete(())));
        assert_eq!(req.method, Some("GET"));
    }

    #[test]
    fn test_http10() {
        let mut headers = HeaderMap::new();
        let mut req = Request::new(&mut headers);
        let result = req.parse_header(b"GET /index.html HTTP/1.0\r\n");
        assert_eq!(result, Ok(Complete(())));
        assert_eq!(req.version, Some(0));
    }

    #[test]
    fn test_root_uri() {
        let mut headers = HeaderMap::new();
        let mut req = Request::new(&mut headers);
        req.parse_header(b"GET / HTTP/1.1\r\n").unwrap();
        assert_eq!(req.path, Some("/"));
    }

    #[test]
    fn test_uri_with_query() {
        let mut headers = HeaderMap::new();
        let mut req = Request::new(&mut headers);
        req.parse_header(b"GET /search?q=hello&lang=zh HTTP/1.1\r\n").unwrap();
        assert_eq!(req.path, Some("/search?q=hello&lang=zh"));
    }

    #[test]
    fn test_bare_cr_in_request_line() {
        let result = parse("GET /index.html HTTP/1.1\r\x00");
        assert_eq!(result.is_ok(), false);
    }

    #[test]
    fn test_post_method() {
        let mut headers = HeaderMap::new();
        let mut req = Request::new(&mut headers);
        let result_type =match req.parse_header(b"POST /submit HTTP/1.1\r\n") {
            Err(_) => false,
            _ => {
                true
            }
        };

        assert_eq!(result_type, true);
        assert_eq!(req.method, Some("POST"));
        assert_eq!(req.path, Some("/submit"));
    }
}