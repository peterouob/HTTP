#[cfg(test)]
mod test {
    use crate::router::error::RouterError;
    use crate::router::radix_tree::{Node, RadixTree};

    #[test]
    fn test_tree_insert_edge_cases() {
        let mut tree: RadixTree<&str> = RadixTree::new(Node::default());

        assert_eq!(tree.insert(b"", "empty"), Err(RouterError::NullKey));
        assert_eq!(tree.insert(b"a", "single_char"), Ok(()));

        assert_eq!(
            tree.insert(b"a", "duplicate_single"),
            Err(RouterError::DuplicateKey)
        );

        assert_eq!(tree.insert(b"user/settings/profile", "long"), Ok(()));

        assert_eq!(tree.insert(b"user/settings", "mid"), Ok(()));

        assert_eq!(tree.insert(b"user", "short"), Ok(()));

        assert_eq!(
            tree.insert(b"user", "dup_short"),
            Err(RouterError::DuplicateKey)
        );

        assert_eq!(
            tree.insert(b"user/settings", "dup_mid"),
            Err(RouterError::DuplicateKey)
        );

        assert_eq!(tree.insert(b"banana", "b1"), Ok(()));
        assert_eq!(tree.insert(b"b", "b2"), Ok(()));
        assert_eq!(tree.insert(b"b", "b2_dup"), Err(RouterError::DuplicateKey));

        assert_eq!(tree.insert(b"user-api", "api"), Ok(()));
        assert_eq!(
            tree.insert(b"user-api", "api_dup"),
            Err(RouterError::DuplicateKey)
        );

        assert_eq!(tree.insert(b"test/api", "t1"), Ok(()));
        assert_eq!(tree.insert(b"best/api", "t2"), Ok(()));
        assert_eq!(
            tree.insert(b"test/api", "t1_dup"),
            Err(RouterError::DuplicateKey)
        );

        assert_eq!(tree.insert(b"a/b/c/d/e", "deep"), Ok(()));
        assert_eq!(tree.insert(b"a/b/c/d", "deep_1"), Ok(()));
        assert_eq!(tree.insert(b"a/b/c", "deep_2"), Ok(()));
        assert_eq!(tree.insert(b"a/b", "deep_3"), Ok(()));

        assert_eq!(
            tree.insert(b"a/b/c/d/e", "dup"),
            Err(RouterError::DuplicateKey)
        );
        assert_eq!(tree.insert(b"a/b", "dup"), Err(RouterError::DuplicateKey));
    }

    #[test]
    fn test_tree_insert_edge_split_cases() {
        let mut tree: RadixTree<&str> = RadixTree::new(Node::default());

        assert_eq!(tree.insert(b"b", "b_first"), Ok(()));
        assert_eq!(tree.insert(b"banana", "b_long_after"), Ok(()));

        assert_eq!(tree.insert(b"romane", "r1"), Ok(()));
        assert_eq!(tree.insert(b"romanus", "r2"), Ok(()));
        assert_eq!(
            tree.insert(b"romane", "r1_dup"),
            Err(RouterError::DuplicateKey)
        );
        assert_eq!(
            tree.insert(b"romanus", "r2_dup"),
            Err(RouterError::DuplicateKey)
        );

        assert_eq!(tree.insert(b"roman", "r3_prefix"), Ok(()));
        assert_eq!(
            tree.insert(b"roman", "r3_dup"),
            Err(RouterError::DuplicateKey)
        );

        assert_eq!(tree.insert(b"abc", "abc"), Ok(()));
        assert_eq!(tree.insert(b"abd", "abd"), Ok(()));
        assert_eq!(tree.insert(b"abe", "abe"), Ok(()));

        assert_eq!(tree.insert(b"ab", "ab_at_split"), Ok(()));
        assert_eq!(tree.insert(b"ab", "ab_dup"), Err(RouterError::DuplicateKey));
    }

    #[test]
    fn test_tree_insert_byte_boundary_cases() {
        let mut tree: RadixTree<&str> = RadixTree::new(Node::default());

        assert_eq!(tree.insert(b"a", "a"), Ok(()));
        assert_eq!(tree.insert(b"b", "b"), Ok(()));
        assert_eq!(tree.insert(b"c", "c"), Ok(()));

        assert_eq!(tree.insert(&[0xFF], "ff"), Ok(()));
        assert_eq!(tree.insert(&[0x80], "80"), Ok(()));
        assert_eq!(
            tree.insert(&[0xFF], "ff_dup"),
            Err(RouterError::DuplicateKey)
        );

        assert_eq!(tree.insert(b"a\0b", "null_mid"), Ok(()));
        assert_eq!(
            tree.insert(b"a\0b", "null_mid_dup"),
            Err(RouterError::DuplicateKey)
        );

        assert_eq!(tree.insert("使用者".as_bytes(), "zh"), Ok(()));
        assert_eq!(tree.insert("使用".as_bytes(), "zh_prefix"), Ok(()));
        assert_eq!(
            tree.insert("使用者".as_bytes(), "zh_dup"),
            Err(RouterError::DuplicateKey)
        );

        assert_eq!(tree.insert(b"/api", "no_slash"), Ok(()));
        assert_eq!(tree.insert(b"/api/", "trailing_slash"), Ok(()));
        assert_eq!(
            tree.insert(b"/api", "no_slash_dup"),
            Err(RouterError::DuplicateKey)
        );

        let long_key = vec![b'x'; 4096];
        assert_eq!(tree.insert(&long_key, "long"), Ok(()));
        assert_eq!(
            tree.insert(&long_key, "long_dup"),
            Err(RouterError::DuplicateKey)
        );
    }

    #[cfg(test)]
    mod find_test {
        use super::*;
        #[test]
        fn test_tree_find_basic_cases() {
            let mut tree: RadixTree<&str> = RadixTree::new(Node::default());

            assert_eq!(tree.find(b"missing"), None);
            assert_eq!(tree.find(b""), None);

            tree.insert(b"a", "a_val").unwrap();
            tree.insert(b"ab", "ab_val").unwrap();
            tree.insert(b"abc", "abc_val").unwrap();

            assert_eq!(tree.find(b"a"), Some(&"a_val"));
            assert_eq!(tree.find(b"ab"), Some(&"ab_val"));
            assert_eq!(tree.find(b"abc"), Some(&"abc_val"));

            assert_eq!(tree.find(b"abcd"), None);
            assert_eq!(tree.find(b"b"), None);
            assert_eq!(tree.find(b""), None);
        }

        #[test]
        fn test_tree_find_split_and_prefix_cases() {
            let mut tree: RadixTree<&str> = RadixTree::new(Node::default());

            tree.insert(b"romane", "r1").unwrap();
            tree.insert(b"romanus", "r2").unwrap();
            tree.insert(b"roman", "r3").unwrap();
            tree.insert(b"romulus", "r4").unwrap();
            tree.insert(b"rubicon", "r5").unwrap();
            tree.insert(b"rubens", "r6").unwrap();

            assert_eq!(tree.find(b"romane"), Some(&"r1"));
            assert_eq!(tree.find(b"romanus"), Some(&"r2"));
            assert_eq!(tree.find(b"roman"), Some(&"r3"));
            assert_eq!(tree.find(b"romulus"), Some(&"r4"));
            assert_eq!(tree.find(b"rubicon"), Some(&"r5"));
            assert_eq!(tree.find(b"rubens"), Some(&"r6"));

            assert_eq!(tree.find(b"r"), None);
            assert_eq!(tree.find(b"ro"), None);
            assert_eq!(tree.find(b"rom"), None);
            assert_eq!(tree.find(b"roma"), None);
            assert_eq!(tree.find(b"romanu"), None);
            assert_eq!(tree.find(b"romanes"), None);
            assert_eq!(tree.find(b"romaneX"), None);
            assert_eq!(tree.find(b"rub"), None);
            assert_eq!(tree.find(b"rubicons"), None);
        }

        #[test]
        fn test_tree_find_deep_nested_cases() {
            let mut tree: RadixTree<&str> = RadixTree::new(Node::default());

            tree.insert(b"a/b/c/d/e", "deep").unwrap();
            tree.insert(b"a/b/c/d", "d4").unwrap();
            tree.insert(b"a/b/c", "d3").unwrap();
            tree.insert(b"a/b", "d2").unwrap();
            tree.insert(b"a", "d1").unwrap();

            assert_eq!(tree.find(b"a"), Some(&"d1"));
            assert_eq!(tree.find(b"a/b"), Some(&"d2"));
            assert_eq!(tree.find(b"a/b/c"), Some(&"d3"));
            assert_eq!(tree.find(b"a/b/c/d"), Some(&"d4"));
            assert_eq!(tree.find(b"a/b/c/d/e"), Some(&"deep"));

            assert_eq!(tree.find(b"a/"), None);
            assert_eq!(tree.find(b"a/b/"), None);
            assert_eq!(tree.find(b"a/b/c/d/e/f"), None);
            assert_eq!(tree.find(b"a/x"), None);
            assert_eq!(tree.find(b"a/b/x"), None);
        }

        #[test]
        fn test_tree_find_branching_cases() {
            let mut tree: RadixTree<&str> = RadixTree::new(Node::default());

            tree.insert(b"abc", "abc").unwrap();
            tree.insert(b"abd", "abd").unwrap();
            tree.insert(b"abe", "abe").unwrap();
            tree.insert(b"ab", "ab").unwrap();

            assert_eq!(tree.find(b"ab"), Some(&"ab"));
            assert_eq!(tree.find(b"abc"), Some(&"abc"));
            assert_eq!(tree.find(b"abd"), Some(&"abd"));
            assert_eq!(tree.find(b"abe"), Some(&"abe"));

            assert_eq!(tree.find(b"a"), None);
            assert_eq!(tree.find(b"abf"), None);
            assert_eq!(tree.find(b"abcd"), None);
            assert_eq!(tree.find(b"ac"), None);

            tree.insert(b"test/api", "t1").unwrap();
            tree.insert(b"best/api", "t2").unwrap();

            assert_eq!(tree.find(b"test/api"), Some(&"t1"));
            assert_eq!(tree.find(b"best/api"), Some(&"t2"));
            assert_eq!(tree.find(b"rest/api"), None);
            assert_eq!(tree.find(b"test"), None);
            assert_eq!(tree.find(b"test/"), None);
            assert_eq!(tree.find(b"test/apix"), None);
        }

        #[test]
        fn test_tree_find_byte_boundary_cases() {
            let mut tree: RadixTree<&str> = RadixTree::new(Node::default());

            tree.insert(&[0xFF], "ff").unwrap();
            tree.insert(&[0x80], "80").unwrap();
            tree.insert(&[0x00], "null").unwrap();
            tree.insert(b"a\0b", "null_mid").unwrap();

            assert_eq!(tree.find(&[0xFF]), Some(&"ff"));
            assert_eq!(tree.find(&[0x80]), Some(&"80"));
            assert_eq!(tree.find(&[0x00]), Some(&"null"));
            assert_eq!(tree.find(b"a\0b"), Some(&"null_mid"));

            assert_eq!(tree.find(&[0xFE]), None);
            assert_eq!(tree.find(&[0x7F]), None);
            assert_eq!(tree.find(b"a\0"), None);
            assert_eq!(tree.find(b"a\0bc"), None);

            tree.insert("使用者".as_bytes(), "zh").unwrap();
            tree.insert("使用".as_bytes(), "zh_prefix").unwrap();

            assert_eq!(tree.find("使用者".as_bytes()), Some(&"zh"));
            assert_eq!(tree.find("使用".as_bytes()), Some(&"zh_prefix"));
            assert_eq!(tree.find("使".as_bytes()), None);
            assert_eq!(tree.find("使用者X".as_bytes()), None);

            let truncated = &"使用者".as_bytes()[..4];
            assert_eq!(tree.find(truncated), None);

            tree.insert(b"/api", "no_slash").unwrap();
            tree.insert(b"/api/", "trailing_slash").unwrap();

            assert_eq!(tree.find(b"/api"), Some(&"no_slash"));
            assert_eq!(tree.find(b"/api/"), Some(&"trailing_slash"));
            assert_eq!(tree.find(b"/ap"), None);
            assert_eq!(tree.find(b"/api/x"), None);

            let long_key = vec![b'x'; 4096];
            tree.insert(&long_key, "long").unwrap();
            assert_eq!(tree.find(&long_key), Some(&"long"));

            let almost = vec![b'x'; 4095];
            assert_eq!(tree.find(&almost), None);

            let too_long = vec![b'x'; 4097];
            assert_eq!(tree.find(&too_long), None);
        }

        #[test]
        fn test_tree_find_empty_and_root_cases() {
            let mut tree: RadixTree<&str> = RadixTree::new(Node::default());

            assert_eq!(tree.find(b""), None);
            assert_eq!(tree.find(b"anything"), None);

            tree.insert(b"x", "x").unwrap();

            assert_eq!(tree.find(b""), None);
            assert_eq!(tree.find(b"x"), Some(&"x"));
            assert_eq!(tree.find(b"y"), None);
            assert_eq!(tree.find(b"xx"), None);
        }
    }
}
