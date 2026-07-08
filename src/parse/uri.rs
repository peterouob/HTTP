use std::collections::HashMap;
use crate::parse::iter::ParseBuffer;
use crate::parse::parse_utils::{uniform_block, Block, BLOCK_SIZE};

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
                    self.raw.go_the_tail();
                    self.query = self.raw.slice();
                    break;
                }
                Some(_) => {
                    self.raw.advance(1);
                    continue;
                }
                None => {
                    self.raw.go_the_tail();
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

#[cfg(target_endian = "little")]
#[inline]
fn get_question_mark_pos_swar(bytes:Block) -> usize {
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
