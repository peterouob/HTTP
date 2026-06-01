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

impl <'a,T> Default for Node<'a,T> {
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
    pub(crate) fn get_edge_mut(&mut self, first_label: u8) -> Option<&mut Node<'a,T>>{
        self.edges
            .iter_mut()
            .find(|e| e.label[0] == first_label)
            .map(|e| &mut e.node)
    }

    #[inline]
    pub(crate) fn get_edge(&self, label: &[u8]) -> Option<&Node<'a,T>>{
        let idx = self.edges.partition_point(|e| e.label < label);
        if idx < self.edges.len() && self.edges[idx].label == label {
            Some(&self.edges[idx].node)
        }else{
            None
        }
    }
}

pub(crate) struct RadixTree<'a,T> {
    root: Node<'a,T>,
    size: usize,
}

impl<T> Debug for RadixTree<'_,T> where T: Debug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RadixTree (size: {})\n└── Root: {:#?}", self.size, self.root)
    }
}

impl <'a,T> RadixTree<'a,T> {
    #[inline]
    pub(crate) fn new(root: Node<'a,T>) -> Self {
        RadixTree { root, size: 0 }
    }

    #[inline]
    pub(crate) fn insert(&mut self, label: &'a [u8], value: T) {
        let search = label;

        if label.is_empty() {
            if let Some(leaf) = self.root.leaf_node.as_mut() {
                let _old_node = mem::replace(&mut leaf.value, value);
            }else{
                self.root.leaf_node = Some(LeafNode { key: label, value });
                self.size += 1;
            }
        }else{
            insert_recursive(&mut self.root, search, value);
            self.size += 1;
        }
    }
}

#[inline]
pub(crate) fn longest_common_prefix<'a>(k1: &'a [u8], k2: &'a [u8]) -> usize {
    k1.iter()
        .zip(k2.iter())
        .take_while(|(a,b)| a == b)
        .count()
}

#[inline]
pub(crate) fn insert_recursive<'a,T>(n: &mut Node<'a,T>, label: &'a [u8],value: T){
    let first = label[..1][0];
    match n.get_edge_mut(first) {
        Some(child) => {
            let lcp = longest_common_prefix(child.prefix, label);

            if lcp == child.prefix.len() {
                insert_recursive(child, &label[lcp..],value);
            }else {
                let split_prefix = &child.prefix[..lcp];
                child.prefix = &child.prefix[lcp..];

                let suffix = &label[lcp..];

                let (split_node_leaf, tail_node_leaf) = if suffix.is_empty() {
                    (Some(LeafNode { key: label, value }),None)
                }else {
                    (None, Some(LeafNode { key: label, value }))
                };

                let mut split_node = Node::new(split_node_leaf, split_prefix);

                let old_node_after_split = mem::take(child);

                split_node.add_edge(Edge{
                    label: old_node_after_split.prefix,
                    node: old_node_after_split,
                });

                if let Some(tail_node_leaf) = tail_node_leaf {
                    split_node.add_edge(Edge{
                        label: suffix,
                        node: Node::new(Some(tail_node_leaf), suffix),
                    });
                }

                *child = split_node;
            }
        },
        None => {
            let leaf_node = LeafNode { key: label, value };
            let prefix = label;
            let edge = Edge {
                label,
                node: Node::new(Some(leaf_node), prefix),
            };
            n.add_edge(edge);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tree_insert() {
        let mut tree: RadixTree<&str> = RadixTree::new(Node::default());
        tree.insert(b"app","one");
        tree.insert(b"apple", "two");
        tree.insert(b"application", "three");
        println!("{:?}", tree);
    }
}