#[macro_export] macro_rules! next {
    ($bytes: ident) => {
        match $bytes.next_byte() {
            Some(b) => b,
            None => return Ok(Status::Partial)
        }
    };
}

#[macro_export] macro_rules! expect {
    ($bytes: ident.peek() == $pat: pat_param => $ret:expr) => {
        expect!(next!($bytes) => $pat => $ret)
    };

    ($e: expr => $pat: pat_param => $ret:expr) => {
        match $e {
            v@$pat => v,
            _ => return $ret,
        }
    };
}

#[macro_export] macro_rules! complete {
    ($e:expr) => {
        match $e? {
            Status::Complete(v) => v,
            Status::Partial => return Ok(Status::Partial),
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::parse::parser::Status;
    use crate::parse::error::ParseResult;
    use crate::parse::iter::ParseBuffer;

    fn run_next_test() -> ParseResult<()> {
        let mut buffer = ParseBuffer::new(b"ab");

        let b = next!(buffer);
        assert_eq!(b, b'a');
        assert_eq!(buffer.cursor, 1);

        let b = next!(buffer);
        assert_eq!(b, b'b');
        assert_eq!(buffer.cursor, 2);

        Ok(Status::Complete(()))
    }

    #[test]
    fn test_next_macro() {
        let result = run_next_test();
        assert_eq!(result, Ok(Status::Complete(())));
    }

    fn run_next_partial_test() -> ParseResult<()> {
        let mut buffer = ParseBuffer::new(b"");
        let _ = next!(buffer);
        Ok(Status::Complete(()))
    }

    #[test]
    fn test_next_partial() {
        let result = run_next_partial_test();
        assert_eq!(result, Ok(Status::Partial));
    }
}