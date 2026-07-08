use crate::parse::parser::{Request, Response};
use crate::parse::uri::Uri;
use crate::router::context::Context;
use crate::router::radix_tree::{Node, RadixTree};

type HandlerFn = fn(&mut Context);


// INFO: I thought the router tree should have longest lifetime, and response has longer lifetime than request
pub struct Engine<'a>{
    get_tree: RadixTree<'a, HandlerFn>,
    post_tree: RadixTree<'a, HandlerFn>,
}

impl<'a> Engine<'a> {
    pub fn new() -> Self {
        Self {
            get_tree: RadixTree::new(Node::default()),
            post_tree: RadixTree::new(Node::default()),
        }
    }

    pub fn get(&mut self, path: &'a [u8], handler: HandlerFn) -> &mut Self {
        let _ = self.get_tree.insert(path, handler);
        self
    }

    pub fn post(&mut self, path: &'a [u8], handler: HandlerFn) -> &mut Self {
        let _ = self.post_tree.insert(path, handler);
        self
    }

    pub(crate) fn dispatch<'h,'b>(&self, req: &Request<'h,'b>, res:&mut Response<'h,'b>) -> Vec<u8> {
        let method = req.method.unwrap_or("GET");
        let path = req.uri.unwrap_or("/").as_bytes();

        let mut uri = Uri::new(path);
        uri.split();
        uri.split_query_string();

        let path = uri.path.unwrap_or(b"/");

        let tree = match method {
            "GET" => &self.get_tree,
            "POST" => &self.post_tree,
            _ => return build_not_found(),
        };

        match tree.find(path) {
            Some(handle) => {
              let mut ctx = Context::new(req, res,uri);
                handle(&mut ctx);
                ctx.res.build()
            },
            None => build_not_found(),
        }
    }
}

fn build_not_found() -> Vec<u8> {
    let mut buf = Vec::with_capacity(1024);
    buf.extend_from_slice(b"HTTP/1.1 404 Not Found\r\n");
    buf.extend_from_slice(b"Content-Length: 0\r\n\r\n");
    buf
}
