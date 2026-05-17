use std::fmt;
use std::fmt::{write, Formatter};
use crate::parse::http_method::Method;

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


#[derive(Debug)]
pub struct Header<'buf> {
    name: &'buf str,
    value: &'buf [u8],
}

#[derive(Debug)]
pub struct Request<'h,'b> {
    pub method: Method<'b>,
    pub path: Option<&'b str>,
    pub version: Option<u8>,
    pub h: &'h mut [Header<'b>]
}

impl <'h,'b> fmt::Display for Request<'h, 'b> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write(f, format_args!("method:{:?}, path:{:?}, version:{:?}, h:{:?}",
                              self.method, self.path, self.version, self.h))
    }
}

#[derive(Debug)]
pub struct Response<'h,'b> {
    pub version: Option<u16>,
    pub status_code: Option<u8>,
    pub msg: Option<&'b str>,
    pub headers: &'h mut [Header<'b>]
}

impl <'h,'b> fmt::Display for Response<'h,'b> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write(f,format_args!("version:{:?}, status_code:{:?}, msg:{:?}, headers:{:?}",
                              self.version, self.status_code, self.msg, self.headers))
    }
}

#[derive(Copy, Clone,PartialOrd, PartialEq,Debug)]
pub enum Status<T>{
    Partial,
    Complete(T),
}

impl<T> Status<T> {
    #[inline]
    pub fn is_complete(&self) -> bool {
        match *self {
            Status::Complete(..) => true,
            Status::Partial => false
        }
    }

    #[inline]
    pub fn is_partial(&self) -> bool {
        match *self {
            Status::Partial => true,
            Status::Complete(..) => false
        }
    }

    #[inline]
    pub fn unwrap(self) -> T {
        match self {
            Status::Complete(t) => t,
            Status::Partial => panic!("Tried to unwrap Status::Partial")
        }
    }
}

pub enum RequestTarget<'buf> {
    Origin {
        path: &'buf [u8],
        query: Option<&'buf [u8]>,
    },
    Absolute(&'buf [u8]),
    Authority(&'buf [u8]),
    Asterisk(Option<&'buf [u8]>),
}

pub enum BodyKind {
    None,
    ContentLength(u64),
    Chunked,
}

pub struct Chunked<'buf> {
    buffer: &'buf [u8],
}

pub(crate) enum CoreRule {
    ALPHA,
    DIGIT,
    HEXDIG,
    SP,
    HTAB,
    WSP,
    VCHAR,
    CTL,
    CRLF,
    CR,
    LF,
    DQUOTE,
}
