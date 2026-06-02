#[cfg(test)]
mod test {
    use crate::parse::error::*;
    use crate::parse::iter::*;
    use crate::parse::parser::*;
    use crate::parse::parser::Status::*;
    use crate::parse::parse_utils::*;

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
            b"", b"H", b"HT", b"HTT", b"HTTP", b"HTTP/", b"HTTP/1", b"HTTP/1.",
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
            "empty URI should be Token error, got {:?}",
            r
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
            matches!(r, Err(ParseError::Token)),
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
