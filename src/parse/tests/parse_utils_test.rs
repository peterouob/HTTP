#[cfg(test)]
mod test {
    use crate::parse::error::*;
    use crate::parse::iter::*;
    use crate::parse::parse_utils::*;
    use crate::parse::parser::Status::*;
    use crate::parse::parser::*;
    use std::collections::HashMap;

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
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/search?q=hello&q=123 HTTP/1.1");
        let r = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(query_map.get(&"q").unwrap()[0], "hello"));
        assert!(matches!(query_map.get(&"q").unwrap()[1], "123"));
    }

    #[test]
    fn test_parse_uri_basic_single_pair() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/search?q=hello HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(query_map.get(&"q").unwrap()[0], "hello"));
    }

    #[test]
    fn test_parse_uri_repeated_key() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/search?q=hello&q=123 HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(query_map.get(&"q").unwrap()[0], "hello"));
        assert!(matches!(query_map.get(&"q").unwrap()[1], "123"));
    }

    #[test]
    fn test_parse_uri_multiple_keys() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/api?id=100&group=admin HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(query_map.get(&"id").unwrap()[0], "100"));
        assert!(matches!(query_map.get(&"group").unwrap()[0], "admin"));
    }

    #[test]
    fn test_parse_uri_no_query() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/search HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(query_map.is_empty());
    }

    #[test]
    fn test_parse_uri_root_path_no_query() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/ HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(query_map.is_empty());
    }

    #[test]
    fn test_parse_uri_question_mark_only() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/search? HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(query_map.is_empty());
    }

    #[test]
    fn test_parse_uri_key_without_equals() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/search?a HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(query_map.get(&"a").unwrap()[0], ""));
    }

    #[test]
    fn test_parse_uri_key_with_empty_value() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/search?a= HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(query_map.get(&"a").unwrap()[0], ""));
    }

    #[test]
    fn test_parse_uri_consecutive_ampersands() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/search?a=1&&b=2 HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(query_map.get(&"a").unwrap()[0], "1"));
        assert!(matches!(query_map.get(&"b").unwrap()[0], "2"));
        assert_eq!(query_map.len(), 2);
    }

    #[test]
    fn test_parse_uri_trailing_ampersand() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/search?a=1& HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(query_map.get(&"a").unwrap()[0], "1"));
        assert_eq!(query_map.len(), 1);
    }

    #[test]
    fn test_parse_uri_leading_ampersand() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/search?&a=1 HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(query_map.get(&"a").unwrap()[0], "1"));
        assert_eq!(query_map.len(), 1);
    }

    #[test]
    fn test_parse_uri_value_contains_equals() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/search?token=abc=def HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(query_map.get(&"token").unwrap()[0], "abc=def"));
    }

    #[test]
    fn test_parse_uri_empty_key_with_value() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/search?=value HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(query_map.get(&"").unwrap()[0], "value"));
    }

    #[test]
    fn test_parse_uri_query_at_block_boundary() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/abcdefg?x=1 HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(query_map.get(&"x").unwrap()[0], "1"));
    }

    #[test]
    fn test_parse_uri_query_just_before_boundary() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/abcdef?x=1 HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(query_map.get(&"x").unwrap()[0], "1"));
    }

    #[test]
    fn test_parse_uri_query_just_after_boundary() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/abcdefgh?x=1 HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(query_map.get(&"x").unwrap()[0], "1"));
    }

    #[test]
    fn test_parse_uri_short_path_below_block_size() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/?a=1 HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(query_map.get(&"a").unwrap()[0], "1"));
    }

    #[test]
    fn test_parse_uri_percent_encoded_value() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/api?name=%E6%B8%AC%E8%A9%A6 HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        assert!(matches!(
            query_map.get(&"name").unwrap()[0],
            "%E6%B8%AC%E8%A9%A6"
        ));
    }

    #[test]
    fn test_parse_uri_three_repeated_keys() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/x?tag=a&tag=b&tag=c HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        let tags = query_map.get(&"tag").unwrap();
        assert_eq!(tags.len(), 3);
        assert!(matches!(tags[0], "a"));
        assert!(matches!(tags[1], "b"));
        assert!(matches!(tags[2], "c"));
    }

    #[test]
    fn test_parse_uri_repeated_key_preserves_order() {
        let mut query_map = HashMap::new();
        let mut buf = ParseBuffer::new(b"/x?q=z&q=a&q=m HTTP/1.1");
        let _ = parse_uri(&mut buf, &mut query_map).unwrap();
        let q = query_map.get(&"q").unwrap();
        assert!(matches!(q[0], "z"));
        assert!(matches!(q[1], "a"));
        assert!(matches!(q[2], "m"));
    }
}
