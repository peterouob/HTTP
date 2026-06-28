use crate::parse::parser::{Request, Response};

pub struct Context<'h,'b,'r>{
    pub req: &'r Request<'h,'b>,
    pub res: &'r mut Response<'h,'b>
}

impl<'h,'b,'r> Context<'h,'b,'r> {
    pub fn new(req: &'r Request<'h,'b>, res: &'r mut Response<'h,'b>) -> Self {
        Self { req, res }
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
}