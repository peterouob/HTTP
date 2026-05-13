pub struct ParseBuffer<'a> {
    buf: &'a [u8],
    start: usize,
    cursor: usize,
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
    pub fn peek_ahead(&self,n: usize) -> Option<u8> {
        self.buf.get(self.cursor + n).copied()
    }

    #[inline]
    pub fn peek_n<const N:usize>(&self) -> Option<[u8;N]> {
        self.buf.get(self.cursor .. self.cursor + N)?.try_into().ok()
    }

    #[inline]
    pub fn advance(&mut self,n: usize) -> anyhow::Result<()>{
        if self.cursor + n > self.buf.len() {
            Err(anyhow::anyhow!("unexpected end of buffer"))
        }else{
            self.cursor += n;
            Ok(())
        }
    }

    #[inline]
    pub fn bump(&mut self) -> anyhow::Result<()> {
        if self.cursor + 1 >= self.buf.len() {
            Err(anyhow::anyhow!("unexpected end of buffer"))
        }else {
            self.cursor += 1;
            Ok(())
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buf.len() - self.cursor
    }

    #[inline]
    pub fn commit(&mut self) {
        self.start = self.cursor;
    }

    #[inline]
    pub fn slice(&mut self) -> &'a [u8] {
        let mut f= false;
        if self.cursor >= self.buf.len() {
            self.cursor = self.buf.len()-1;
            f = true
        }
        let slice = &self.buf[self.start..=self.cursor];
        if f {
            self.cursor += 1;
        }
        self.commit();
        slice
    }

}

impl<'a> Iterator for ParseBuffer<'a> {
    type Item = u8;

    #[inline]
    fn next(&mut self) -> Option<u8> {
        if self.cursor < self.buf.len() {
            let b = self.buf[self.cursor];
            self.cursor += 1;
            Some(b)
        }else{
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_buffer_range() {
        let mut buf = ParseBuffer::new(b"foo");
        buf.advance(1).unwrap();
        assert_eq!(buf.slice(), b"fo");
        buf.advance(2).unwrap();
        assert_eq!(buf.slice(), b"oo");
        assert_eq!(buf.advance(1).is_err(), true);
    }

    #[test]
    fn test_buffer_iter() {
        let mut buf = ParseBuffer::new(b"fof");
        assert_eq!(buf.peek(), Some(b'f'));
        assert_eq!(buf.peek_n(), Some(*b"fof"));
        assert_eq!(buf.peek_ahead(2), Some(b'f'));
        assert_eq!(buf.len(), "fof".as_bytes().len());
        assert_eq!(buf.next(), Some(b'f'));
        assert_eq!(buf.start,0);
        assert_eq!(buf.cursor,1);
        assert_eq!(buf.slice(), b"fo");
        assert_eq!(buf.next(), Some(b'o'));
        assert_eq!(buf.cursor,2);
        assert_eq!(buf.slice(), b"of");
        assert_eq!(buf.next(), Some(b'f'));
        assert_eq!(buf.cursor,3);
    }

    #[test]
    fn test_buffer_bump() {
        let mut buf = ParseBuffer::new(b"foo");
        assert_eq!(buf.bump().is_ok(), true);
        assert_eq!(buf.cursor,1);
        assert_eq!(buf.advance(2).is_ok(),true);
        assert_eq!(buf.bump().is_err(), true);
    }
}