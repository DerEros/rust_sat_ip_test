use hyper::Request;
use std::convert::From;

pub struct RenderableRequest(pub Request<()>);

impl From<RenderableRequest> for String {
    fn from(req: RenderableRequest) -> Self {

        format!("{} {} {}\n{}\r\n",
            1, 2, 3, 4
        )
    }
}