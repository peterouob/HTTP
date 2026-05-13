use std::collections::HashMap;

pub struct Request<'buf> {
    buffer: &'buf [u8],
}

pub struct HeaderMap<'buf> {
    headers: HashMap<&'buf [u8], &'buf [u8]>,
}

pub struct HeaderName<'buf> {
    name: &'buf [u8],
}

pub struct HeaderValue<'buf> {
    value: &'buf [u8],
}

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

/*
┌──────────────────────────────────────────┐
│ start-line                          CRLF │  ← 一行
├──────────────────────────────────────────┤
│ field-line 1                        CRLF │
│ field-line 2                        CRLF │  ← *( ... )，零或多行
│ ...                                      │
│ field-line N                        CRLF │
├──────────────────────────────────────────┤
│                                     CRLF │  ← 空行，標記 header 結束
├──────────────────────────────────────────┤
│                                          │
│            message-body                  │  ← 可選，長度由 header 決定(不在message)
│                                          │
└──────────────────────────────────────────┘

CRLF -> 1. end of start-line
        2. end of each field line
        3. empty line after headers section
*/
