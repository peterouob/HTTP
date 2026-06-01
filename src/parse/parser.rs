use crate::parse::error::{ParseError, ParseResult};
use crate::parse::iter::ParseBuffer;
use crate::parse::parse_utils::{
    match_header_name_vector, match_header_value, parse_method, parse_reason, parse_status_code,
    parse_uri, parse_version, skip_empty_line,
};
use crate::parse::tchar::{is_header_name_token, is_header_value};
use crate::{complete, expect, newline};
use crate::{next, space};
use std::collections::HashMap;
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
pub struct HeaderMap<'buf> {
    pub(crate) header: HashMap<&'buf str, &'buf [u8]>,
}

impl<'buf> HeaderMap<'buf> {
    pub fn new() -> Self {
        HeaderMap {
            header: HashMap::new(),
        }
    }

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

    pub fn parse_header(&mut self, bytes: &'b [u8]) -> Result<Status<()>, ParseError> {
        let mut bytes = ParseBuffer::new(bytes);
        complete!(skip_empty_line(&mut bytes));
        let method = complete!(parse_method(&mut bytes));
        self.method = Some(method);
        self.path = Some(complete!(parse_uri(&mut bytes)));
        self.version = Some(complete!(parse_version(&mut bytes)));

        newline!(bytes);

        complete!(parse_header_iter(&mut bytes, self.headers));
        Ok(Status::Complete(()))
    }
}

pub fn parse_header_iter<'a>(
    bytes: &mut ParseBuffer<'a>,
    headers: &mut HeaderMap<'a>,
) -> ParseResult<usize> {
    let start = bytes.as_ref().as_ptr() as usize;
    let mut result = Err(ParseError::TooManyHeaders);

    loop {
        let b = next!(bytes);

        if b == b'\r' {
            expect!(bytes.peek()==b'\n'=>Err(ParseError::NewLine));
            let end = bytes.as_ref().as_ptr() as usize;
            result = Ok(Status::Complete(end - start));
            break;
        }

        if b == b'\n' {
            let end = bytes.as_ref().as_ptr() as usize;
            result = Ok(Status::Complete(end - start));
            break;
        }

        if !is_header_name_token(b) {
            result = Err(ParseError::HeaderName);
            break;
        }

        let header_name: &str = 'name: loop {
            match_header_name_vector(bytes);
            let b = next!(bytes);

            let bslice = bytes.sub_slice(1);

            // TODO: avoid unwrap to get value
            let name = str::from_utf8(bslice.unwrap()).unwrap();

            if b == b':' {
                break 'name name;
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
                } else if b != b'\n' {
                    return Err(ParseError::HeaderValue);
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
                } else if b == b'\n' {
                    1
                } else {
                    return Err(ParseError::HeaderValue);
                };

                break 'value bytes.sub_slice(skip_num).unwrap();
            }
        };

        let header_value = if let Some(last_visible) = header_value_slice
            .iter()
            .rposition(|b| *b != b' ' && *b != b'\t' && *b != b'\r' && *b != b'\n')
        {
            &header_value_slice[..last_visible + 1]
        } else {
            header_value_slice
        };

        headers.insert(header_name, header_value);
    }
    result
}

#[derive(Debug)]
pub struct Response<'h, 'b> {
    pub version: Option<u8>,
    pub status_code: Option<u16>,
    pub reason: Option<&'b str>,
    pub headers: &'h mut HeaderMap<'b>,
}

impl<'h, 'b> fmt::Display for Response<'h, 'b> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write(
            f,
            format_args!(
                "version:{:?}, status_code:{:?}, msg:{:?}, headers:{:?}",
                self.version, self.status_code, self.headers, self.reason
            ),
        )
    }
}

impl<'h, 'b> Response<'h, 'b> {
    #[inline]
    pub fn new(headers: &'h mut HeaderMap<'b>) -> Response<'h, 'b> {
        Response {
            version: None,
            status_code: None,
            reason: None,
            headers,
        }
    }

    // HTTP/1.1[space]200
    #[inline]
    pub fn parse_header(&mut self, bytes: &'b [u8]) -> Result<Status<()>, ParseError> {
        let mut bytes = ParseBuffer::new(bytes);
        complete!(skip_empty_line(&mut bytes));
        self.version = Some(complete!(parse_version(&mut bytes)));
        space!(bytes or ParseError::Version);
        self.status_code = Some(complete!(parse_status_code(&mut bytes)));

        match next!(bytes) {
            b' ' => {
                bytes.slice();
                self.reason = Some(complete!(parse_reason(&mut bytes)));
            }
            b'\r' => {
                expect!(bytes.peek()==b'\n'=>Err(ParseError::NewLine));
                bytes.slice();
                self.reason = Some("");
            }
            b'\n' => {
                bytes.slice();
                self.reason = Some("");
            }
            _ => return Err(ParseError::Reason),
        }

        complete!(parse_header_iter(&mut bytes, self.headers));
        Ok(Status::Complete(()))
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
