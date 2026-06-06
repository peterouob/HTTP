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

            assert_eq!(tree.find(b"missing").unwrap(), None);
            assert_eq!(tree.find(b"").unwrap(), None);

            tree.insert(b"a", "a_val").unwrap();
            tree.insert(b"ab", "ab_val").unwrap();
            tree.insert(b"abc", "abc_val").unwrap();

            assert_eq!(tree.find(b"a").unwrap(), Some(&"a_val"));
            assert_eq!(tree.find(b"ab").unwrap(), Some(&"ab_val"));
            assert_eq!(tree.find(b"abc").unwrap(), Some(&"abc_val"));

            assert_eq!(tree.find(b"abcd").unwrap(), None);
            assert_eq!(tree.find(b"b").unwrap(), None);
            assert_eq!(tree.find(b"").unwrap(), None);
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

            assert_eq!(tree.find(b"romane").unwrap(), Some(&"r1"));
            assert_eq!(tree.find(b"romanus").unwrap(), Some(&"r2"));
            assert_eq!(tree.find(b"roman").unwrap(), Some(&"r3"));
            assert_eq!(tree.find(b"romulus").unwrap(), Some(&"r4"));
            assert_eq!(tree.find(b"rubicon").unwrap(), Some(&"r5"));
            assert_eq!(tree.find(b"rubens").unwrap(), Some(&"r6"));

            assert_eq!(tree.find(b"r").unwrap(), None);
            assert_eq!(tree.find(b"ro").unwrap(), None);
            assert_eq!(tree.find(b"rom").unwrap(), None);
            assert_eq!(tree.find(b"roma").unwrap(), None);
            assert_eq!(tree.find(b"romanu").unwrap(), None);
            assert_eq!(tree.find(b"romanes").unwrap(), None);
            assert_eq!(tree.find(b"romaneX").unwrap(), None);
            assert_eq!(tree.find(b"rub").unwrap(), None);
            assert_eq!(tree.find(b"rubicons").unwrap(), None);
        }

        #[test]
        fn test_tree_find_deep_nested_cases() {
            let mut tree: RadixTree<&str> = RadixTree::new(Node::default());

            tree.insert(b"a/b/c/d/e", "deep").unwrap();
            tree.insert(b"a/b/c/d", "d4").unwrap();
            tree.insert(b"a/b/c", "d3").unwrap();
            tree.insert(b"a/b", "d2").unwrap();
            tree.insert(b"a", "d1").unwrap();

            assert_eq!(tree.find(b"a").unwrap(), Some(&"d1"));
            assert_eq!(tree.find(b"a/b").unwrap(), Some(&"d2"));
            assert_eq!(tree.find(b"a/b/c").unwrap(), Some(&"d3"));
            assert_eq!(tree.find(b"a/b/c/d").unwrap(), Some(&"d4"));
            assert_eq!(tree.find(b"a/b/c/d/e").unwrap(), Some(&"deep"));

            assert_eq!(tree.find(b"a/").unwrap(), None);
            assert_eq!(tree.find(b"a/b/").unwrap(), None);
            assert_eq!(tree.find(b"a/b/c/d/e/f").unwrap(), None);
            assert_eq!(tree.find(b"a/x").unwrap(), None);
            assert_eq!(tree.find(b"a/b/x").unwrap(), None);
        }

        #[test]
        fn test_tree_find_branching_cases() {
            let mut tree: RadixTree<&str> = RadixTree::new(Node::default());

            tree.insert(b"abc", "abc").unwrap();
            tree.insert(b"abd", "abd").unwrap();
            tree.insert(b"abe", "abe").unwrap();
            tree.insert(b"ab", "ab").unwrap();

            assert_eq!(tree.find(b"ab").unwrap(), Some(&"ab"));
            assert_eq!(tree.find(b"abc").unwrap(), Some(&"abc"));
            assert_eq!(tree.find(b"abd").unwrap(), Some(&"abd"));
            assert_eq!(tree.find(b"abe").unwrap(), Some(&"abe"));

            assert_eq!(tree.find(b"a").unwrap(), None);
            assert_eq!(tree.find(b"abf").unwrap(), None);
            assert_eq!(tree.find(b"abcd").unwrap(), None);
            assert_eq!(tree.find(b"ac").unwrap(), None);

            tree.insert(b"test/api", "t1").unwrap();
            tree.insert(b"best/api", "t2").unwrap();

            assert_eq!(tree.find(b"test/api").unwrap(), Some(&"t1"));
            assert_eq!(tree.find(b"best/api").unwrap(), Some(&"t2"));
            assert_eq!(tree.find(b"rest/api").unwrap(), None);
            assert_eq!(tree.find(b"test").unwrap(), None);
            assert_eq!(tree.find(b"test/").unwrap(), None);
            assert_eq!(tree.find(b"test/apix").unwrap(), None);
        }

        #[test]
        fn test_tree_find_byte_boundary_cases() {
            let mut tree: RadixTree<&str> = RadixTree::new(Node::default());

            tree.insert(&[0xFF], "ff").unwrap();
            tree.insert(&[0x80], "80").unwrap();
            tree.insert(&[0x00], "null").unwrap();
            tree.insert(b"a\0b", "null_mid").unwrap();

            assert_eq!(tree.find(&[0xFF]).unwrap(), Some(&"ff"));
            assert_eq!(tree.find(&[0x80]).unwrap(), Some(&"80"));
            assert_eq!(tree.find(&[0x00]).unwrap(), Some(&"null"));
            assert_eq!(tree.find(b"a\0b").unwrap(), Some(&"null_mid"));

            assert_eq!(tree.find(&[0xFE]).unwrap(), None);
            assert_eq!(tree.find(&[0x7F]).unwrap(), None);
            assert_eq!(tree.find(b"a\0").unwrap(), None);
            assert_eq!(tree.find(b"a\0bc").unwrap(), None);

            tree.insert("使用者".as_bytes(), "zh").unwrap();
            tree.insert("使用".as_bytes(), "zh_prefix").unwrap();

            assert_eq!(tree.find("使用者".as_bytes()).unwrap(), Some(&"zh"));
            assert_eq!(tree.find("使用".as_bytes()).unwrap(), Some(&"zh_prefix"));
            assert_eq!(tree.find("使".as_bytes()).unwrap(), None);
            assert_eq!(tree.find("使用者X".as_bytes()).unwrap(), None);

            let truncated = &"使用者".as_bytes()[..4];
            assert_eq!(tree.find(truncated).unwrap(), None);

            tree.insert(b"/api", "no_slash").unwrap();
            tree.insert(b"/api/", "trailing_slash").unwrap();

            assert_eq!(tree.find(b"/api").unwrap(), Some(&"no_slash"));
            assert_eq!(tree.find(b"/api/").unwrap(), Some(&"trailing_slash"));
            assert_eq!(tree.find(b"/ap").unwrap(), None);
            assert_eq!(tree.find(b"/api/x").unwrap(), None);

            let long_key = vec![b'x'; 4096];
            tree.insert(&long_key, "long").unwrap();
            assert_eq!(tree.find(&long_key).unwrap(), Some(&"long"));

            let almost = vec![b'x'; 4095];
            assert_eq!(tree.find(&almost).unwrap(), None);

            let too_long = vec![b'x'; 4097];
            assert_eq!(tree.find(&too_long).unwrap(), None);
        }

        #[test]
        fn test_tree_find_empty_and_root_cases() {
            let mut tree: RadixTree<&str> = RadixTree::new(Node::default());

            assert_eq!(tree.find(b"").unwrap(), None);
            assert_eq!(tree.find(b"anything").unwrap(), None);

            tree.insert(b"x", "x").unwrap();

            assert_eq!(tree.find(b"").unwrap(), None);
            assert_eq!(tree.find(b"x").unwrap(), Some(&"x"));
            assert_eq!(tree.find(b"y").unwrap(), None);
            assert_eq!(tree.find(b"xx").unwrap(), None);
        }
    }
}
