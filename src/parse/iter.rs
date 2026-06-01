use std::fmt;
use std::fmt::write;

pub struct ParseBuffer<'a> {
    pub buf: &'a [u8],
    start: usize,
    pub cursor: usize,
}

impl<'a> fmt::Display for ParseBuffer<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write(f, format_args!("buf:{:?}", str::from_utf8(self.buf)))
    }
}

impl<'a> ParseBuffer<'a> {
    #[inline]
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            start: 0,
            cursor: 0,
        }
    }

    #[inline]
    pub fn peek(&self) -> Option<u8> {
        self.buf.get(self.cursor).copied()
    }

    #[inline]
    pub fn peek_pos_n(&self, n: usize) -> Option<u8> {
        let n = self.cursor.checked_add(n)?;
        self.buf.get(n).copied()
    }

    #[inline]
    pub fn peek_after_cursor<const N: usize>(&self) -> Option<[u8; N]> {
        let n = self.cursor.checked_add(N)?;
        self.buf.get(self.cursor..n)?.try_into().ok()
    }

    // When advance return None, that mean it need more bytes
    // So this is not error but need to handle to the status
    #[inline]
    pub fn advance(&mut self, n: usize) -> Option<()> {
        let advance_cursor = self.cursor.checked_add(n)?;
        if advance_cursor > self.buf.len() {
            return None;
        }

        self.cursor = advance_cursor;
        Some(())
    }

    #[inline]
    pub fn len_remain(&self) -> usize {
        self.buf.len() - self.cursor
    }

    #[inline]
    pub fn commit(&mut self) {
        self.start = self.cursor;
    }

    #[inline]
    pub fn slice(&mut self) -> Option<&'a [u8]> {
        debug_assert!(self.start <= self.cursor, "start > cursor");
        debug_assert!(self.cursor <= self.buf.len(), "cursor > buf.len()");

        let s = self.buf.get(self.start..self.cursor)?;
        self.start = self.cursor;
        Some(s)
    }

    #[inline]
    pub fn sub_slice(&mut self, sub_n: usize) -> Option<&'a [u8]> {
        debug_assert!(self.start <= self.cursor, "start > cursor");
        debug_assert!(self.cursor <= self.buf.len(), "cursor >= buf.len()");

        let end = self.cursor.checked_sub(sub_n)?;

        if self.start > end {
            return None;
        }

        let s = &self.buf[self.start..end];

        self.start = self.cursor;
        Some(s)
    }

    #[inline]
    pub fn next_byte(&mut self) -> Option<u8> {
        let b = self.buf.get(self.cursor).copied()?;
        self.cursor += 1;
        Some(b)
    }
}

impl AsRef<[u8]> for ParseBuffer<'_> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.buf.get(self.start..self.cursor).unwrap_or(&[])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_buffer() {
        let mut buf = ParseBuffer::new(b"hello world");
        assert_eq!(buf.len_remain(), 11);
        buf = ParseBuffer::new(b"");
        assert_eq!(buf.len_remain(), 0);
        assert_eq!(buf.start, 0);
        assert_eq!(buf.cursor, 0);
    }

    #[test]
    fn test_buffer_peek() {
        let mut buf = ParseBuffer::new(b"hello");
        assert_eq!(buf.peek(), Some(b'h'));
        buf.advance(2).unwrap();
        assert_eq!(buf.peek(), Some(b'l'));
        buf.commit();
        buf.advance(3).unwrap();
        assert_eq!(buf.peek(), None);
        buf.commit();
        assert_eq!(buf.advance(1).unwrap_or(()), ());

        buf = ParseBuffer::new(b"hl");
        buf.advance(2).unwrap();
        assert_eq!(buf.peek(), None);

        buf = ParseBuffer::new(b"");
        assert_eq!(buf.peek(), None);

        buf = ParseBuffer::new(b"hello world");
        buf.advance(2).unwrap();
        assert_eq!(buf.peek_after_cursor(), Some(*b"llo world"));
        assert_eq!(buf.peek_pos_n(5), Some(b'o'));
        assert_eq!(buf.peek_pos_n(1000), None)
    }

    #[test]
    fn test_buffer_slice() {
        let mut buf = ParseBuffer::new(b"GET /path");
        buf.advance(3).unwrap();
        assert_eq!(buf.len_remain(), 6);
        assert_eq!(buf.slice(), Some(b"GET".as_slice()));
        assert_eq!(buf.start, buf.cursor);
        buf.advance(1).unwrap();
        buf.commit();
        buf.advance(5).unwrap();
        assert_eq!(buf.slice(), Some(b"/path".as_slice()));
    }

    #[test]
    fn test_buffer() {
        let mut buf = ParseBuffer::new(b"GET /path HTTP/1.1\r\n");
        assert_eq!(buf.peek_after_cursor::<4>(), Some(*b"GET "));
        assert_eq!(buf.peek_pos_n(3), Some(b' '));
        buf.advance(3).unwrap();
        assert_eq!(buf.slice(), Some(b"GET".as_slice()));
        buf.advance(1).unwrap();
        assert_eq!(buf.cursor, 4);
        assert_eq!(buf.peek(), Some(b'/'));
        buf.commit();
        while let Some(b) = buf.peek() {
            if b == b' ' {
                break;
            }
            buf.advance(1).unwrap();
        }

        assert_eq!(buf.slice(), Some(b"/path".as_slice()));
        buf.advance(1).unwrap();
        buf.commit();
        while let Some(b) = buf.peek() {
            if b == b'\r' {
                assert_eq!(buf.slice(), Some(b"HTTP/1.1".as_slice()));
                buf.commit();
                buf.advance(2).unwrap();
                assert_eq!(buf.slice(), Some(b"\r\n".as_slice()));
                break;
            }
            buf.advance(1).unwrap();
        }
    }

    #[test]
    fn test_as_ref() {
        let mut buf = ParseBuffer::new(b"GET /path HTTP/1.1\r\n");
        buf.advance(2);
        assert_eq!(buf.as_ref(), b"GE");
        assert_eq!(buf.as_ref().len(), 2);

        let buf = ParseBuffer::new(b"");
        assert_eq!(buf.as_ref(), b"");
        assert_eq!(buf.as_ref().len(), 0);
    }

    #[test]
    fn test_sub_slice() {
        let mut buf = ParseBuffer::new(b"12345678");
        buf.advance(8).unwrap();
        assert_eq!(buf.sub_slice(1), Some(b"1234567".as_slice()));

        let mut buf = ParseBuffer::new(b"12345678");
        buf.advance(8).unwrap();
        assert_eq!(buf.sub_slice(2), Some(b"123456".as_slice()));
    }
}
