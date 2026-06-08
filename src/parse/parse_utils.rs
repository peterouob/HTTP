use crate::parse::error::{ParseError, ParseResult};
use crate::parse::iter::ParseBuffer;
use crate::parse::parser::Status;
use crate::parse::parser::Status::Complete;
use crate::parse::tchar::{is_header_name_token, is_header_value, is_method_token, is_url_token};
use crate::{expect, next};
use std::collections::HashMap;

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
pub(crate) fn skip_space_line(bytes: &mut ParseBuffer) -> ParseResult<()> {
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
pub(crate) fn parse_status_code(bytes: &mut ParseBuffer<'_>) -> Result<Status<u16>, ParseError> {
    let hundreds = expect!(bytes.peek() == b'0'..=b'9' => Err(ParseError::StatusCode));
    let ten = expect!(bytes.peek() == b'0'..=b'9' => Err(ParseError::StatusCode));
    let one = expect!(bytes.peek() == b'0'..=b'9' => Err(ParseError::StatusCode));

    Ok(Complete(
        (hundreds - b'0') as u16 * 100 + (ten - b'0') as u16 * 10 + (one - b'0') as u16,
    ))
}

#[inline]
pub(crate) fn parse_reason<'a>(bytes: &mut ParseBuffer<'a>) -> Result<Status<&'a str>, ParseError> {
    loop {
        let b = next!(bytes);

        if b == b'\r' {
            expect!(bytes.peek() == b'\n'=> Err(ParseError::NewLine));
            let bytes = bytes.sub_slice(2).unwrap();
            let slice = str::from_utf8(bytes).unwrap();
            return Ok(Complete(slice));
        } else if b == b'\n' {
            let bytes = bytes.sub_slice(1).unwrap();
            let slice = str::from_utf8(bytes).unwrap();
            return Ok(Complete(slice));
        } else if !(b == 0x09 || b == b' ' || (0x21..=0x7E).contains(&b) || b >= 0x80) {
            return Err(ParseError::ReasonInvalidCode);
        }
    }
}

#[inline]
pub(crate) fn parse_method<'a>(bytes: &mut ParseBuffer<'a>) -> Result<Status<&'a str>, ParseError> {
    const GET: [u8; 4] = *b"GET ";
    const POST: [u8; 4] = *b"POST";
    match bytes.peek_after_cursor::<4>() {
        Some(GET) => {
            let method = {
                bytes.advance(4);
                str::from_utf8(bytes.sub_slice(1).unwrap()).unwrap()
            };
            Ok(Complete(method))
        }
        Some(POST) => {
            let method = {
                bytes.advance(5);
                str::from_utf8(bytes.sub_slice(1).unwrap()).unwrap()
            };
            Ok(Complete(method))
        }
        _ => parse_token(bytes),
    }
}

#[inline]
pub(crate) fn parse_token<'a>(bytes: &mut ParseBuffer<'a>) -> ParseResult<&'a str> {
    let b = next!(bytes);
    if !is_method_token(b) {
        return Err(ParseError::Token);
    }

    loop {
        let b = next!(bytes);
        if b == b' ' {
            let s = bytes.sub_slice(1).unwrap();
            return Ok(Complete(str::from_utf8(s).unwrap()));
        } else if !is_method_token(b) {
            return Err(ParseError::Token);
        }
    }
}

#[inline]
pub(crate) fn parse_version(bytes: &mut ParseBuffer) -> ParseResult<u8> {
    if let Some(eight) = bytes.peek_after_cursor::<8>() {
        const H10: u64 = u64::from_be_bytes(*b"HTTP/1.0");
        const H11: u64 = u64::from_be_bytes(*b"HTTP/1.1");

        bytes.advance(8);

        return match u64::from_be_bytes(eight) {
            H10 => Ok(Complete(0)),
            H11 => Ok(Complete(1)),
            _ => Err(ParseError::Version),
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
pub(crate) fn parse_uri<'a>(bytes: &mut ParseBuffer<'a>) -> ParseResult<&'a str> {
    let start = bytes.cursor;
    match_uri_token(bytes);
    let end = bytes.cursor;

    if end == start {
        return Err(ParseError::Token);
    }

    if next!(bytes) == b' ' {
        let slice = match bytes.sub_slice(1) {
            Some(slice) => slice,
            _ => return Err(ParseError::Token),
        };

        match str::from_utf8(slice) {
            Ok(uri) => Ok(Complete(uri)),
            Err(_) => Err(ParseError::Token),
        }
    } else {
        Err(ParseError::Token)
    }
}

pub struct Uri<'a> {
    raw: ParseBuffer<'a>,
    pub path: Option<&'a [u8]>,
    pub query: Option<&'a [u8]>,
    pub query_map: HashMap<&'a str, Vec<&'a str>>,
}

impl<'a> Uri<'a> {
    #[inline]
    pub fn new(u8_raw: &'a [u8]) -> Self {
        Uri {
            raw: ParseBuffer::new(u8_raw),
            path: None,
            query: None,
            query_map: HashMap::new(),
        }
    }

    #[inline]
    pub fn split(&mut self) {
        loop {
            if let Some(b8) = self.raw.peek_after_cursor::<BLOCK_SIZE>() {
                let n = get_question_mark_pos_swar(b8);

                self.raw.advance(n);

                if n == BLOCK_SIZE {
                    continue;
                }
            };
            match self.raw.peek() {
                Some(0x3F) => {
                    self.path = self.raw.slice();
                    self.raw.advance(1);
                    self.raw.commit();
                    self.raw.to_the_tail();
                    self.query = self.raw.slice();
                    break;
                }
                Some(_) => {
                    self.raw.advance(1);
                    continue;
                }
                None => {
                    self.raw.to_the_tail();
                    self.path = self.raw.slice();
                    self.query = None;
                }
            }

            break;
        }
    }

    #[inline]
    pub fn split_query_string(&mut self) {
        // url: https://example.com/api/users?id=100&group=admin
        // uri: https://example.com/api/users
        // query: id=100&group=admin

        let Some(query) = self.query else {
            return;
        };

        for pair in query.split(|&b| b == b'&') {
            if pair.is_empty() {
                continue;
            }

            let (key, val) = match pair.iter().position(|&b| b == b'=') {
                Some(pos) => (&pair[..pos], &pair[pos + 1..]),
                None => (pair, &[][..]),
            };

            let k = str::from_utf8(key).unwrap_or_default();
            let v = str::from_utf8(val).unwrap_or_default();

            self.query_map.entry(k).or_default().push(v);
        }
    }
}

#[inline]
pub(crate) fn match_uri_token(b: &mut ParseBuffer) {
    loop {
        if let Some(b8) = b.peek_after_cursor::<BLOCK_SIZE>() {
            let n = match_uri_char(b8);
            b.advance(n);

            if n == BLOCK_SIZE {
                continue;
            }
        }

        if let Some(byte) = b.peek()
            && is_url_token(byte)
        {
            b.advance(1);
            continue;
        }

        break;
    }
}

#[cfg(target_endian = "little")]
#[inline]
fn get_question_mark_pos_swar(bytes: Block) -> usize {
    let question_mark = uniform_block(0x3F);
    let low_bits = uniform_block(0x01);
    let high_bits = uniform_block(0x80);

    let b = usize::from_ne_bytes(bytes);

    let b_xor = b ^ question_mark;
    let eq_question = b_xor.wrapping_sub(low_bits) & !b_xor & high_bits;

    if eq_question == 0 {
        return BLOCK_SIZE;
    }

    (eq_question.trailing_zeros() >> 3) as usize
}

#[cfg(target_endian = "little")]
#[inline]
fn get_colon_pos(bytes: Block) -> usize {
    let colon = uniform_block(0x3A);
    let low_bits = uniform_block(0x01);
    let high_bits = uniform_block(0x80);

    let b = usize::from_ne_bytes(bytes);

    let b_xor = b ^ colon;
    let eq_colon = b_xor.wrapping_sub(low_bits) & !b_xor & high_bits;

    if eq_colon == 0 {
        return BLOCK_SIZE;
    }

    (eq_colon.trailing_zeros() >> 3) as usize
}

const fn uniform_block(b: u8) -> usize {
    u64::from_ne_bytes([b; 8]) as usize
}

const BLOCK_SIZE: usize = size_of::<usize>();
type Block = [u8; BLOCK_SIZE];

// TODO: reference httparse use swar algorithm instead the function
// 1. match_uri_char
// 2. match_header_value_char
#[inline]
fn match_uri_char(b: Block) -> usize {
    for (i, &b) in b.iter().enumerate() {
        if b < 33 || b == 127 {
            return i;
        }
    }
    BLOCK_SIZE
}

#[inline]
pub fn match_header_value(bytes: &mut ParseBuffer) {
    loop {
        if let Some(b8) = bytes.peek_after_cursor::<BLOCK_SIZE>() {
            let n = match_header_value_char(b8);
            bytes.advance(n);

            if n == BLOCK_SIZE {
                continue;
            }
        }

        if let Some(byte) = bytes.peek()
            && is_header_value(byte)
        {
            bytes.advance(1);
            continue;
        }

        break;
    }
}

#[inline]
fn match_header_value_char(b: Block) -> usize {
    for (i, &b) in b.iter().enumerate() {
        if b < 32 || b == 127 {
            return i;
        }
    }
    BLOCK_SIZE
}

#[inline]
pub fn match_header_name_vector(bytes: &mut ParseBuffer) {
    while let Some(b) = bytes.peek_after_cursor::<BLOCK_SIZE>() {
        let n = match_full_block_size(is_header_name_token, b);
        bytes.advance(n);
        if n != BLOCK_SIZE {
            return;
        }
    }

    bytes.advance(match_block_size(is_header_name_token, bytes.as_ref()));
}

#[inline]
fn match_full_block_size(f: impl Fn(u8) -> bool, b: Block) -> usize {
    for (i, &b) in b.iter().enumerate() {
        if !f(b) {
            return i;
        }
    }
    BLOCK_SIZE
}

#[inline]
fn match_block_size(f: impl Fn(u8) -> bool, block: &[u8]) -> usize {
    for (i, &b) in block.iter().enumerate() {
        if !f(b) {
            return i;
        }
    }
    block.len()
}
