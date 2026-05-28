use crate::next;
use crate::expect;
use crate::parse::error::{ParseError, ParseResult};
use crate::parse::iter::ParseBuffer;
use crate::parse::parser::Status;
use crate::parse::parser::Status::Complete;
use crate::parse::tchar::is_url_token;

#[inline]
pub(crate) fn skip_empty_line(bytes: &mut ParseBuffer) -> ParseResult<()> {
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
                return Ok(Complete(()));
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
                return Ok(Complete(()));
            }
            None => return Ok(Status::Partial),
        }
    }
}

#[inline]
pub(crate) fn parse_method<'a>(bytes: &mut ParseBuffer<'a>)  -> Result<Status<&'a str>, ParseError> {
    const GET: [u8; 4] = *b"GET ";
    const POST: [u8;4] = *b"POST";
    match bytes.peek_after_cursor::<4>() {
        Some(GET) => {
            let method = {
                bytes.advance(3);
                str::from_utf8(bytes.slice().unwrap()).unwrap()
            };
            Ok(Complete(method))
        }
        Some(POST) => {
            let method = {
                bytes.advance(4);
                str::from_utf8(bytes.slice().unwrap()).unwrap()
            };
            Ok(Complete(method))
        }
        _ => {
            todo!()
        }
    }
}

#[inline]
pub(crate) fn parse_version(bytes: &mut ParseBuffer) -> ParseResult<u8> {
    if let Some(eight) = bytes.peek_after_cursor::<8>() {
        const H10:u64 = u64::from_be_bytes(*b"HTTP/1.0");
        const H11:u64 = u64::from_be_bytes(*b"HTTP/1.1");

        bytes.advance(8);

        return match u64::from_be_bytes(eight) {
            H10 => Ok(Complete(0)),
            H11 => Ok(Complete(1)),
            _ => Err(ParseError::Version)
        };
    }

    expect!(bytes.peek() == b'H' => Err(ParseError::Version));
    expect!(bytes.peek() == b'T' => Err(ParseError::Version));
    expect!(bytes.peek() == b'T' => Err(ParseError::Version));
    expect!(bytes.peek() == b'P' => Err(ParseError::Version));
    expect!(bytes.peek() == b'/' => Err(ParseError::Version));
    expect!(bytes.peek() == b'1' => Err(ParseError::Version));
    expect!(bytes.peek() == b'.' => Err(ParseError::Version));

    Ok(Status::Partial)
}

#[inline]
pub(crate) fn parse_uri<'a>(bytes: &mut ParseBuffer<'a>) -> ParseResult<&'a str>{
    let start = bytes.cursor;
    match_uri_token(bytes);
    let end = bytes.cursor;

    if end == start {
        return Err(ParseError::Token)
    }

    if next!(bytes) == b' ' {

        let slice = match bytes.slice().and_then(|s| s.split_last()) {
            Some((_last,slice)) => slice,
            _ =>return Err(ParseError::Token)
        };

        match str::from_utf8(slice) {
            Ok(uri) => Ok(Complete(uri)),
            Err(_) => Err(ParseError::Token)
        }

    }else {
        Err(ParseError::Token)
    }
}

#[inline]
pub(crate) fn match_uri_token(b: &mut ParseBuffer) {
    loop {
        if let Some(b8) = b.peek_after_cursor::<BLOCK_SIZE>() {
            let n = match_uri_char(b8);
            b.advance(n);

            if n == 8 {
                continue;
            }
        }

        if let Some(byte) = b.peek() {
            if is_url_token(byte) {
                b.advance(1);
                continue;
            }
        }

        break;
    }
}

const BLOCK_SIZE: usize = size_of::<usize>();
type Block = [u8; BLOCK_SIZE];

#[inline]
fn match_uri_char(b: Block) -> usize{
    for (i, &b) in b.iter().enumerate() {
        if b < 33 || b == 127 {
            return i;
        }
    }
    BLOCK_SIZE
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_skip_empty_line() {
        let mut buf = ParseBuffer::new(b"Hello");
        assert_eq!(skip_empty_line(&mut buf).unwrap(), Complete(()));
        assert_eq!(buf.cursor, 0);

        let mut buf = ParseBuffer::new(b"\r\nHello");
        assert_eq!(skip_empty_line(&mut buf).unwrap(), Complete(()));
        assert_eq!(buf.cursor, 2);

        let mut buf = ParseBuffer::new(b"\nHello");
        assert_eq!(skip_empty_line(&mut buf).unwrap(), Complete(()));
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
        assert_eq!(skip_space_line(&mut buf).unwrap(), Complete(()));

        let mut buf = ParseBuffer::new(b" ");
        assert_eq!(skip_space_line(&mut buf).unwrap(), Status::Partial);
    }

    #[test]
    fn test_parse_method() {
        let mut buf = ParseBuffer::new(b"GET / HTTP/1.1");
        assert_eq!(parse_method(&mut buf).unwrap(), Complete("GET"));
        let mut buf = ParseBuffer::new(b"POST / HTTP/1.1");
        assert_eq!(parse_method(&mut buf).unwrap(), Complete("POST"));

    }

    #[test]
    fn test_parse_version() {
        let mut buf = ParseBuffer::new(b"HTTP/1.1");
        let r = parse_version(&mut buf).unwrap();
        assert_eq!(r, Complete(1));

        let mut buf = ParseBuffer::new(b"HTTP/1.0");
        let r = parse_version(&mut buf).unwrap();
        assert_eq!(r, Complete(0));

        let cases: &[&[u8]] = &[
            b"",
            b"H",
            b"HT",
            b"HTT",
            b"HTTP",
            b"HTTP/",
            b"HTTP/1",
            b"HTTP/1.",
        ];

        for &input in cases {
            let mut buf = ParseBuffer::new(input);
            let result = parse_version(&mut buf).unwrap();
            assert_eq!(
                result,
                Status::Partial,
                "input {:?} should be Partial",
                input
            );
        }
    }

    #[test]
    fn test_parse_version_invalid() {
        let error_cases: &[&[u8]] = &[
            b"XTTP/1.1",
            b"HXTP/1.1",
            b"HTXP/1.1",
            b"HTTX/1.1",
            b"HTTPX1.1",
            b"HTTP/X.1",
            b"HTTP/1X1",
        ];

        for &input in error_cases {
            let mut buf = ParseBuffer::new(input);
            let result = parse_version(&mut buf);
            assert!(
                matches!(result, Err(ParseError::Version)),
                "input {:?} should be Err(Version)",
                input
            );
        }
    }

    #[test]
    fn test_parse_uri() {
        let mut buf = ParseBuffer::new(b"/index.html HTTP/1.1");
        let r = parse_uri(&mut buf).unwrap();
        assert_eq!(r, Complete("/index.html"));
        assert_eq!(buf.peek(), Some(b'H'));

        let mut buf = ParseBuffer::new(b"/ HTTP/1.1");
        let r = parse_uri(&mut buf).unwrap();
        assert_eq!(r, Complete("/"));

        let mut buf = ParseBuffer::new(b"/search?q=hello HTTP/1.1");
        let r = parse_uri(&mut buf).unwrap();
        assert_eq!(r, Complete("/search?q=hello"));

        let mut buf = ParseBuffer::new(b"/a/b/c/d/e/f/g/h HTTP/1.1");
        let r = parse_uri(&mut buf).unwrap();
        assert_eq!(r, Complete("/a/b/c/d/e/f/g/h"));

        let mut buf = ParseBuffer::new(b"/path-v2.0_test~!$& HTTP/1.1");
        let r = parse_uri(&mut buf).unwrap();
        assert_eq!(r, Complete("/path-v2.0_test~!$&"));

        let mut buf = ParseBuffer::new(b"");
        let r = parse_uri(&mut buf);
        assert_eq!(r, Err(ParseError::Token));

        let mut buf = ParseBuffer::new(b" HTTP/1.1");
        let r = parse_uri(&mut buf);
        assert!(
            matches!(r, Err(ParseError::Token)),
            "empty URI should be Token error, got {:?}", r
        );

        let mut buf = ParseBuffer::new(b"/pa\x01th HTTP/1.1");
        let r = parse_uri(&mut buf);
        assert!(
            matches!(r, Err(ParseError::Token)),
            "CTL in URI should be Token error"
        );

        let mut buf = ParseBuffer::new(b"/pa\x7fth HTTP/1.1");
        let r = parse_uri(&mut buf);
        assert!(
            matches!(r,Err(ParseError::Token)),
            "DEL in URI should be Token error"
        );

        let mut buf = ParseBuffer::new(b"/path HTTP/1.1");
        parse_uri(&mut buf).unwrap();
        assert_eq!(buf.cursor, 6);
        assert_eq!(buf.peek(), Some(b'H'));

        let mut buf = ParseBuffer::new(b"/abcdefg HTTP/1.1");
        let r = parse_uri(&mut buf).unwrap();
        assert_eq!(r, Complete("/abcdefg"));
        assert_eq!(buf.peek(), Some(b'H'));

        let mut buf = ParseBuffer::new(b"/abc def HTTP/1.1");
        let r = parse_uri(&mut buf).unwrap();
        assert_eq!(r, Complete("/abc"));
        assert_eq!(buf.peek(), Some(b'd'));
    }
}
