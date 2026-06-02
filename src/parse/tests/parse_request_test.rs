use crate::parse::error::*;
use crate::parse::parser::*;
#[test]
fn test_get_with_common_headers() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let input = "GET /index.html HTTP/1.1\r\n\
                     Host: example.com\r\n\
                     User-Agent: Mozilla/5.0\r\n\
                     Accept: text/html\r\n\
                     Connection: keep-alive\r\n\r\n";

    let result = req.parse_header(input.as_bytes());
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(req.method, Some("GET"));
    assert_eq!(req.uri, Some("/index.html"));
    assert_eq!(req.version, Some(1));
    assert_eq!(
        req.headers.header.get("Host"),
        Some(&b"example.com"[..].as_ref())
    );
    assert_eq!(
        req.headers.header.get("User-Agent"),
        Some(&b"Mozilla/5.0"[..].as_ref())
    );
    assert_eq!(
        req.headers.header.get("Accept"),
        Some(&b"text/html"[..].as_ref())
    );
    assert_eq!(
        req.headers.header.get("Connection"),
        Some(&b"keep-alive"[..].as_ref())
    );
}

#[test]
fn test_post_with_content_headers() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let input = "POST /submit HTTP/1.1\r\n\
                     Host: example.com\r\n\
                     Content-Type: application/json\r\n\
                     Content-Length: 42\r\n\r\n";

    let result = req.parse_header(input.as_bytes());
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(req.method, Some("POST"));
    assert_eq!(req.uri, Some("/submit"));
    assert_eq!(
        req.headers.header.get("Content-Type"),
        Some(&b"application/json"[..].as_ref())
    );
    assert_eq!(
        req.headers.header.get("Content-Length"),
        Some(&b"42"[..].as_ref())
    );
}

#[test]
fn test_http10_request() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let input = "GET / HTTP/1.0\r\n\
                     Host: example.com\r\n\r\n";

    let result = req.parse_header(input.as_bytes());
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(req.version, Some(0));
}

#[test]
fn test_header_with_ows() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let input = "GET / HTTP/1.1\r\n\
                     Host:   example.com   \r\n\r\n";

    let result = req.parse_header(input.as_bytes());
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(
        req.headers.header.get("Host"),
        Some(&b"example.com"[..].as_ref())
    );
}

#[test]
fn test_multiple_headers_same_name() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let input = "GET / HTTP/1.1\r\n\
                     Accept: text/html\r\n\
                     Accept: application/json\r\n\r\n";

    let result = req.parse_header(input.as_bytes());
    assert_eq!(result, Ok(Status::Complete(())));
}

#[test]
fn test_leading_crlf_tolerated() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let input = "\r\nGET /index.html HTTP/1.1\r\n\
                     Host: example.com\r\n\r\n";

    let result = req.parse_header(input.as_bytes());
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(req.method, Some("GET"));
}

#[test]
fn test_partial_empty() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let result = req.parse_header(b"");
    assert_eq!(result, Ok(Status::Partial));
}

#[test]
fn test_partial_incomplete_request_line() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let result = req.parse_header(b"GET /index.html");
    assert_eq!(result, Ok(Status::Partial));
}

#[test]
fn test_partial_incomplete_header() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let result = req.parse_header(b"GET / HTTP/1.1\r\nHost: example");
    assert_eq!(result, Ok(Status::Partial));
}

#[test]
fn test_partial_missing_final_crlf() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let result = req.parse_header(b"GET / HTTP/1.1\r\nHost: example.com\r\n");
    assert_eq!(result, Ok(Status::Partial));
}

#[test]
fn test_error_bare_cr_in_header_value() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let result = req.parse_header(b"GET / HTTP/1.1\r\nHost: exam\rple\r\n\r\n");
    assert!(matches!(result, Err(ParseError::NewLine)));
}

#[test]
fn test_empty_header_value() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let result = req.parse_header(b"GET / HTTP/1.1\r\nX-Empty:\r\n\r\n");
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(req.headers.header.get("X-Empty"), Some(&b""[..].as_ref()));
}

#[test]
fn test_no_headers() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let result = req.parse_header(b"GET / HTTP/1.1\r\n\r\n");
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(req.method, Some("GET"));
}

#[test]
fn test_error_empty_header_name() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let result = req.parse_header(b"GET / HTTP/1.1\r\n: value\r\n\r\n");
    assert!(matches!(result, Err(ParseError::HeaderName)));
}

#[test]
fn test_partial_mid_value() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let result = req.parse_header(b"GET / HTTP/1.1\r\nHost: exam");
    assert_eq!(result, Ok(Status::Partial));
}

#[test]
fn test_bare_lf_line_ending() {
    let mut headers = HeaderMap::new();
    let mut req = Request::new(&mut headers);
    let result = req.parse_header(b"GET / HTTP/1.1\nHost: example.com\n\n");
    assert_eq!(result, Ok(Status::Complete(())));
}
