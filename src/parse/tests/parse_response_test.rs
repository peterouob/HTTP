use crate::parse::error::*;
use crate::parse::parser::*;
#[test]
fn test_simple_200_ok() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let input = "HTTP/1.1 200 OK\r\n\
                 Content-Type: text/html\r\n\
                 Content-Length: 42\r\n\r\n";

    let result = resp.parse_header(input.as_bytes());
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(resp.version, Some(1));
    assert_eq!(resp.status_code, Some(200));
    assert_eq!(resp.reason, Some("OK"));
}

#[test]
fn test_404_not_found() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let input = "HTTP/1.1 404 Not Found\r\n\
                 Content-Length: 0\r\n\r\n";

    let result = resp.parse_header(input.as_bytes());
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(resp.status_code, Some(404));
    assert_eq!(resp.reason, Some("Not Found"));
}

#[test]
fn test_500_with_multiword_reason() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let input = "HTTP/1.1 500 Internal Server Error\r\n\r\n";

    let result = resp.parse_header(input.as_bytes());
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(resp.status_code, Some(500));
    assert_eq!(resp.reason, Some("Internal Server Error"));
}

#[test]
fn test_http10_response() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let input = "HTTP/1.0 200 OK\r\n\r\n";

    let result = resp.parse_header(input.as_bytes());
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(resp.version, Some(0));
}

#[test]
fn test_empty_reason_with_crlf() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let input = "HTTP/1.1 204\r\n\r\n";

    let result = resp.parse_header(input.as_bytes());
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(resp.status_code, Some(204));
    assert_eq!(resp.reason, Some(""));
}

#[test]
fn test_empty_reason_with_space() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let input = "HTTP/1.1 204 \r\n\r\n";

    let result = resp.parse_header(input.as_bytes());
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(resp.reason, Some(""));
}

#[test]
fn test_redirect_with_location() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let input = "HTTP/1.1 301 Moved Permanently\r\n\
                 Location: https://example.com/new\r\n\
                 Content-Length: 0\r\n\r\n";

    let result = resp.parse_header(input.as_bytes());
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(resp.status_code, Some(301));
    assert_eq!(
        resp.headers.header.get("Location"),
        Some(&b"https://example.com/new"[..].as_ref())
    );
}

#[test]
fn test_partial_empty() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let result = resp.parse_header(b"");
    assert_eq!(result, Ok(Status::Partial));
}

#[test]
fn test_partial_only_version() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let result = resp.parse_header(b"HTTP/1.1");
    assert_eq!(result, Ok(Status::Partial));
}

#[test]
fn test_partial_no_reason() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let result = resp.parse_header(b"HTTP/1.1 200");
    assert_eq!(result, Ok(Status::Partial));
}

#[test]
fn test_partial_no_final_crlf() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let result = resp.parse_header(b"HTTP/1.1 200 OK\r\n");
    assert_eq!(result, Ok(Status::Partial));
}

#[test]
fn test_error_invalid_version() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let result = resp.parse_header(b"HTTP/2.0 200 OK\r\n\r\n");
    assert!(matches!(result, Err(ParseError::Version)));
}

#[test]
fn test_error_lowercase_http() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let result = resp.parse_header(b"http/1.1 200 OK\r\n\r\n");
    assert!(matches!(result, Err(ParseError::Version)));
}

#[test]
fn test_error_status_code_too_short() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let result = resp.parse_header(b"HTTP/1.1 20 OK\r\n\r\n");
    assert!(matches!(result.is_err(), true));
}

#[test]
fn test_error_status_code_too_long() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let result = resp.parse_header(b"HTTP/1.1 2000 OK\r\n\r\n");
    assert!(matches!(result.is_err(), true));
}

#[test]
fn test_error_non_digit_status_code() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let result = resp.parse_header(b"HTTP/1.1 2X0 OK\r\n\r\n");
    assert!(matches!(result.is_err(), true));
}

#[test]
fn test_error_no_space_after_version() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let result = resp.parse_header(b"HTTP/1.1\t200 OK\r\n\r\n");
    assert!(matches!(result, Err(ParseError::Version)));
}

#[test]
fn test_error_reason_with_ctl() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let result = resp.parse_header(b"HTTP/1.1 200 O\x01K\r\n\r\n");
    assert!(matches!(result, Err(ParseError::ReasonInvalidCode)));
}

#[test]
fn test_response_with_multiple_headers() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let input = "HTTP/1.1 200 OK\r\n\
                 Server: nginx/1.18\r\n\
                 Date: Mon, 01 Jan 2024 00:00:00 GMT\r\n\
                 Content-Type: application/json\r\n\
                 Transfer-Encoding: chunked\r\n\r\n";

    let result = resp.parse_header(input.as_bytes());
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(
        resp.headers.header.get("Server"),
        Some(&b"nginx/1.18"[..].as_ref())
    );
    assert_eq!(
        resp.headers.header.get("Transfer-Encoding"),
        Some(&b"chunked"[..].as_ref())
    );
}

#[test]
fn test_response_no_headers() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let input = "HTTP/1.1 304 Not Modified\r\n\r\n";

    let result = resp.parse_header(input.as_bytes());
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(resp.status_code, Some(304));
}

#[test]
fn test_status_code_100() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let result = resp.parse_header(b"HTTP/1.1 100 Continue\r\n\r\n");
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(resp.status_code, Some(100));
}

#[test]
fn test_status_code_999() {
    let mut headers = HeaderMap::new();
    let mut resp = Response::new(&mut headers);
    let result = resp.parse_header(b"HTTP/1.1 999 Custom\r\n\r\n");
    assert_eq!(result, Ok(Status::Complete(())));
    assert_eq!(resp.status_code, Some(999));
}
