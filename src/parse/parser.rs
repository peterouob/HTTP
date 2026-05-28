use std::collections::HashMap;
use crate::{complete, expect};
use crate::parse::error::{ParseError, ParseResult};
use crate::parse::iter::ParseBuffer;
use crate::parse::parse_utils::{skip_empty_line,parse_method};
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

#[derive(Debug,Eq,PartialEq)]
pub struct HeaderMap<'buf> {
    header: HashMap<&'buf str, &'buf [u8]>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Request<'h, 'b> {
    pub method: Option<&'b str>,
    pub path: Option<&'b str>,
    pub version: Option<u8>,
    pub headers: &'h mut Vec<HeaderMap<'b>>,
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
    pub fn new(headers: &'h mut Vec<HeaderMap<'b>>) -> Request<'h, 'b> {
        Request {
            method: None,
            path: None,
            version: None,
            headers,
        }
    }

    fn parse_header(&mut self,headers: &mut HeaderMap, bytes: &'b [u8])-> Result<Status<()>, ParseError> {
        let origin_len = bytes.len();
        let mut bytes = ParseBuffer::new(bytes);
        complete!(skip_empty_line(&mut bytes));
        let method = complete!(parse_method(&mut bytes));
        self.method = Some(method);

        Ok(Status::Complete(()))
    }
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

