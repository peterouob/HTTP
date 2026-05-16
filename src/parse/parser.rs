use std::fmt;
use std::fmt::{write, Formatter};
use crate::parse::error::ParseError;
use crate::parse::http_method::Method;
use crate::parse::surface::Header;


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

pub enum Status<T>{
    Partial,
    Complete(T),
}