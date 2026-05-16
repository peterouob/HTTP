macro_rules! byte_map {
    ($(|)? $p:pat) => {{
        const fn make_map() -> [bool;256] {
            let mut ret = [false;256];
            let mut i = 0;
            while i < 256 {
                ret[i] = matches!(i as u8, $p);
                i += 1;
            }
            ret
        }
        make_map()
    }}
}

#[cfg(test)]
mod tests {
    use std::any::type_name;

    fn type_of<T>(_: T) -> &'static str {
        type_name::<T>()
    }

    #[test]
    fn test_is_digit()  {
        let map = byte_map!(
    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' |
    b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' |  b'*' | b'+' |
    b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~');

        assert_eq!(type_of(map), "[bool; 256]");
    }
}
