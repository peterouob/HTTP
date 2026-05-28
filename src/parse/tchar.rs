macro_rules! byte_map {
    ($(|)? $p:pat) => {{
        const fn make_map() -> [bool; 256] {
            let mut ret = [false; 256];
            let mut i = 0;
            while i < 256 {
                ret[i] = matches!(i as u8, $p);
                i += 1;
            }
            ret
        }
        make_map()
    }};
}

#[inline]
pub(crate) fn is_method_token(b: u8) -> bool {
    matches!(b,b'A'..=b'Z' | b'a'..=b'z')
}

static TOKEN_MAP: [bool; 256] = byte_map!(
    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' |
    b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' |  b'*' | b'+' |
    b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
);

#[inline]
pub(crate) fn is_header_name(b: u8) -> bool {
    TOKEN_MAP[b as usize]
}

static URL_MAP: [bool; 256] = byte_map!(
    b'!'..=0x7E | 0x80..=0xFF
);

#[inline]
pub(crate) fn is_url_token(b: u8) -> bool {
    URL_MAP[b as usize]
}

static HEADER_VALUE_MAP: [bool; 256] = byte_map!(
  b'\t' | b' '..=0x7E | 0x80..=0xFF
);

#[inline]
pub(crate) fn is_header_value(b: u8) -> bool {
    HEADER_VALUE_MAP[b as usize]
}

#[cfg(test)]
mod tests {
    use std::any::type_name;
    use super::*;
    fn type_of<T>(_: T) -> &'static str {
        type_name::<T>()
    }

    #[test]
    fn test_is_digit() {
        let map = TOKEN_MAP;
        assert_eq!(type_of(map), "[bool; 256]");
    }
}
