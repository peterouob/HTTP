use crate::parse::parser::{Request, Response};
use crate::router::radix_tree::{Node, RadixTree};

type HandleFunc = fn(Request);

pub struct Engine<'a> {
    get_tree: RadixTree<'a, HandleFunc>,
    post_tree: RadixTree<'a, HandleFunc>,
}

impl<'a> Engine<'a> {
    pub fn new() -> Self {
        Self {
            get_tree: RadixTree::new(Node::default()),
            post_tree: RadixTree::new(Node::default()),
        }
    }

    pub fn get(&mut self, path: &'a [u8], func: HandleFunc) -> &mut Self {
        let _ = self.get_tree.insert(path, func);
        self
    }

    pub fn post(&mut self, path: &'a [u8], func: HandleFunc) -> &mut Self {
        let _ = self.post_tree.insert(path, func);
        self
    }

    pub(crate) fn dispatch(&mut self, req: &Request) -> Vec<u8> {
        let method = req.method.unwrap_or("GET");
        let path = req.uri.unwrap_or("/").as_bytes();

        let tree = match method {
            "GET" => &self.get_tree,
            "POST" => &self.post_tree,
            _ => todo!(),
        };

        todo!()
    }
}
