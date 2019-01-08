use hyper::Request;
use std::convert::From;
use http::header::{HeaderMap, HeaderValue};

pub struct RenderableRequest(pub Request<()>);

impl From<RenderableRequest> for Vec<u8> {
    fn from(RenderableRequest(req): RenderableRequest) -> Self {
        format!("{} {} {:?}\n{}\r\n",
            req.method(),
            req.uri(),
            req.version(),
            String::from(RenderableHeaderMap(req.headers())),
        ).into_bytes()
    }
}

pub struct RenderableHeaderMap<'a>(pub &'a HeaderMap<HeaderValue>);

impl <'a> From<RenderableHeaderMap<'a>> for String {
    fn from(RenderableHeaderMap(header_map): RenderableHeaderMap) -> Self {
        let mut header_str = String::default();

        for header in header_map.iter() {
            let next_header = format!("{}: {}\n",
                                      header.0.as_str().to_uppercase(),
                                      header.1.to_str().unwrap());
            header_str.push_str(next_header.as_ref());
        }

        header_str
    }
}