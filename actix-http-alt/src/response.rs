use std::{error, io};

use bytes::Bytes;
use http::{header, status::StatusCode, Response};

use super::body::ResponseBody;

/// Helper trait for convert Service::Error type to Service::Response.
// TODO: Add method to modify status code.
pub trait ResponseError<Res> {
    fn response_error(e: Self) -> Res;
}

// implement ResponseError for common error types.

impl<B> ResponseError<Response<ResponseBody<B>>> for Box<dyn error::Error> {
    fn response_error(this: Self) -> Response<ResponseBody<B>> {
        internal_error(this.to_string().as_bytes())
    }
}

impl<B> ResponseError<Response<ResponseBody<B>>> for io::Error {
    fn response_error(this: Self) -> Response<ResponseBody<B>> {
        internal_error(this.to_string().as_bytes())
    }
}

fn internal_error<B>(buf: &[u8]) -> Response<ResponseBody<B>> {
    // TODO: write this to bytes mut directly.
    let bytes = Bytes::copy_from_slice(buf);
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/plain; charset=utf-8"),
        )
        .body(ResponseBody::Bytes { bytes })
        .unwrap()
}