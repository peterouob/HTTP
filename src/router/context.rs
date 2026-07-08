use crate::parse::parser::{Request, Response};
use crate::parse::uri::Uri;

pub struct Context<'h,'b,'r>{
    pub req: &'r Request<'h,'b>,
    pub res: &'r mut Response<'h,'b>,
    uri: Uri<'b>
}

impl<'h,'b,'r> Context<'h,'b,'r> {
    pub fn new(req: &'r Request<'h,'b>, res: &'r mut Response<'h,'b>,uri: Uri<'b>) -> Self {
        Self { req, res, uri }
    }

    pub fn status(&mut self, code: u16) -> &mut Self{
        self.res.status_code = Some(code);
        self.res.reason = match code {
            200 => Option::from("OK"),
            201 =>Option::from( "Created"),
            204 =>Option::from( "No Content"),
            301 =>Option::from( "Moved Permanently"),
            302 =>Option::from( "Found"),
            400 =>Option::from( "Bad Request"),
            401 =>Option::from( "Unauthorized"),
            403 =>Option::from( "Forbidden"),
            404 =>Option::from( "Not Found"),
            500 =>Option::from( "Internal) Server Error"),
            _ => Option::from("Unknown"),
        };
        self
    }

    pub fn header(&mut self, name: &'static str, value: &'static str) -> &mut Self {
        self.res.headers.insert(name, value.as_bytes());
        self
    }

    pub fn html(&mut self, html: &'static str){
        self.header("Content-Type", "text/html; charset=utf-8");
        self.res.body.extend_from_slice(html.as_bytes());
    }

    pub fn string(&mut self, s: &'static str){
        self.header("Content-Type", "text/plain; charset=utf-8");
        self.res.body.extend_from_slice(s.as_bytes());
    }

    pub fn query(&self,key: &str) -> &'b str{
        self.uri.query_map.get(key)
            .and_then(|v| v.first())
            .copied()
            .unwrap()
    }

    pub fn key_query_or_not(&self, key: &str) -> Option<&'b str> {
        self.uri.query_map.get(key)
            .and_then(|v| v.first())
            .copied()
    }

    pub fn query_array(&self, key: &str) -> &[&'b str] {
        self.uri.query_map.get(key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn path(&self) -> &'b [u8] {
        self.uri.path.unwrap_or(b"/")
    }
}