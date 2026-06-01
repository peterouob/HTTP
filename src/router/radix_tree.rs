use crate::router::error::RouterError;
use crate::router::error::RouterResult;
use std::fmt::Debug;
use std::{fmt, mem};

#[derive(Debug)]
pub(crate) struct LeafNode<'a, T> {
    key: &'a [u8],
    // TODO: value will store the function which do the router thing when it access
    value: T,
}

#[derive(Debug)]
pub(crate) struct Edge<'a, T> {
    label: &'a [u8],
    node: Node<'a, T>,
}

#[derive(Debug)]
pub(crate) struct Node<'a, T> {
    leaf_node: Option<LeafNode<'a, T>>,
    edges: Vec<Edge<'a, T>>,
    prefix: &'a [u8],
}

impl<'a, T> Default for Node<'a, T> {
    #[inline]
    fn default() -> Self {
        Node {
            leaf_node: None,
            edges: Vec::new(),
            prefix: &[],
        }
    }
}

impl<'a, T> Node<'a, T> {
    #[inline]
    pub(crate) fn new(leaf_node: Option<LeafNode<'a, T>>, prefix: &'a [u8]) -> Self {
        Node {
            leaf_node,
            edges: Vec::new(),
            prefix,
        }
    }

    #[inline]
    pub(crate) fn add_edge(&mut self, edge: Edge<'a, T>) {
        let idx = self.edges.partition_point(|e| e.label < edge.label);
        self.edges.insert(idx, edge);
    }

    #[inline]
    pub(crate) fn get_edge_mut(&mut self, first_label: u8) -> Option<&mut Node<'a, T>> {
        self.edges
            .iter_mut()
            .find(|e| e.label[0] == first_label)
            .map(|e| &mut e.node)
    }

    #[inline]
    pub(crate) fn get_edge(&self, label: &[u8]) -> Option<&Node<'a, T>> {
        let idx = self.edges.partition_point(|e| e.label < label);
        if idx < self.edges.len() && self.edges[idx].label == label {
            Some(&self.edges[idx].node)
        } else {
            None
        }
    }
}

pub(crate) struct RadixTree<'a, T> {
    root: Node<'a, T>,
    size: usize,
}

impl<T> Debug for RadixTree<'_, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RadixTree (size: {})\n└── Root: {:#?}",
            self.size, self.root
        )
    }
}

impl<'a, T> RadixTree<'a, T> {
    #[inline]
    pub(crate) fn new(root: Node<'a, T>) -> Self {
        RadixTree { root, size: 0 }
    }

    #[inline]
    pub(crate) fn insert(&mut self, label: &'a [u8], value: T) -> RouterResult<()> {
        check_not_null(label)?;
        let search = label;

        insert_recursive(&mut self.root, search, value)?;
        self.size += 1;
        Ok(())
    }
}

#[inline]
pub(crate) fn check_not_null(key: &[u8]) -> RouterResult<()> {
    if key.is_empty() {
        Err(RouterError::NullKey)
    } else {
        Ok(())
    }
}

#[inline]
pub(crate) fn longest_common_prefix<'a>(k1: &'a [u8], k2: &'a [u8]) -> usize {
    k1.iter().zip(k2.iter()).take_while(|(a, b)| a == b).count()
}

#[inline]
pub(crate) fn insert_recursive<'a, T>(
    n: &mut Node<'a, T>,
    label: &'a [u8],
    value: T,
) -> RouterResult<()> {
    let first = label.get(0).copied().unwrap();
    match n.get_edge_mut(first) {
        Some(child) => {
            let lcp = longest_common_prefix(child.prefix, label);

            if lcp == child.prefix.len() {
                let next_label = &label[lcp..];

                if next_label.is_empty() {
                    if child.leaf_node.is_some() {
                        return Err(RouterError::DuplicateKey);
                    }
                    child.leaf_node = Some(LeafNode { key: label, value });
                    return Ok(());
                }
                insert_recursive(child, &label[lcp..], value)
            } else {
                let split_prefix = &child.prefix[..lcp];
                let suffix = &label[lcp..];

                child.prefix = &child.prefix[lcp..];

                let (split_node_leaf, tail_node_leaf) = if suffix.is_empty() {
                    (Some(LeafNode { key: label, value }), None)
                } else {
                    (None, Some(LeafNode { key: label, value }))
                };

                let mut split_node = Node::new(split_node_leaf, split_prefix);

                let old_node_after_split = mem::take(child);

                split_node.add_edge(Edge {
                    label: old_node_after_split.prefix,
                    node: old_node_after_split,
                });

                if let Some(tail_node_leaf) = tail_node_leaf {
                    split_node.add_edge(Edge {
                        label: suffix,
                        node: Node::new(Some(tail_node_leaf), suffix),
                    });
                }

                let _ = mem::replace(child, split_node);
                Ok(())
            }
        }
        None => {
            let leaf_node = LeafNode { key: label, value };
            let prefix = label;
            let edge = Edge {
                label,
                node: Node::new(Some(leaf_node), prefix),
            };
            n.add_edge(edge);
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
}
