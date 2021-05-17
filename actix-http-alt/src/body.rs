use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_core::stream::{LocalBoxStream, Stream};
use pin_project_lite::pin_project;

use super::error::BodyError;

/// A unified request body type for different http protocols.
/// This enables one service type to handle multiple http protocols.
pub enum RequestBody {
    H1(super::h1::RequestBody),
    #[cfg(feature = "http2")]
    H2(super::h2::RequestBody),
    #[cfg(feature = "http3")]
    H3(super::h3::RequestBody),
}

impl Stream for RequestBody {
    type Item = Result<Bytes, BodyError>;

    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            Self::H1(body) => Pin::new(body).poll_next(cx),
            #[cfg(feature = "http2")]
            Self::H2(body) => Pin::new(body).poll_next(cx),
            #[cfg(feature = "http3")]
            Self::H3(body) => Pin::new(body).poll_next(cx),
        }
    }
}

pub type StreamBody = LocalBoxStream<'static, Result<Bytes, BodyError>>;

pin_project! {
    /// A unified response body type.
    /// Generic type is for custom pinned response body.
    #[project = ResponseBodyProj]
    #[project_replace = ResponseBodyProjReplace]
    pub enum ResponseBody<B = StreamBody> {
        None,
        Bytes {
            bytes: Bytes
        },
        Stream {
            #[pin]
            stream: B
        },
    }
}

impl<B, E> ResponseBody<B>
where
    B: Stream<Item = Result<Bytes, E>>,
    BodyError: From<E>,
{
    // TODO: use std::stream::Stream::next when it's added.
    #[doc(hidden)]
    /// Helper for StreamExt::next method.
    pub fn next(self: Pin<&mut Self>) -> Next<'_, B> {
        Next { stream: self }
    }

    pub fn is_eof(&self) -> bool {
        match *self {
            Self::None => true,
            Self::Bytes { ref bytes } => bytes.is_empty(),
            Self::Stream { .. } => false,
        }
    }

    /// Construct a new Stream variant of ResponseBody
    pub fn stream(stream: B) -> Self {
        Self::Stream { stream }
    }
}

pub struct Next<'a, B: Stream> {
    stream: Pin<&'a mut ResponseBody<B>>,
}

impl<B, E> Future for Next<'_, B>
where
    B: Stream<Item = Result<Bytes, E>>,
    BodyError: From<E>,
{
    type Output = Option<Result<Bytes, BodyError>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().stream.as_mut().poll_next(cx)
    }
}

impl<B, E> Stream for ResponseBody<B>
where
    B: Stream<Item = Result<Bytes, E>>,
    BodyError: From<E>,
{
    type Item = Result<Bytes, BodyError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.as_mut().project() {
            ResponseBodyProj::None => Poll::Ready(None),
            ResponseBodyProj::Bytes { .. } => match self.project_replace(ResponseBody::None) {
                ResponseBodyProjReplace::Bytes { bytes } => Poll::Ready(Some(Ok(bytes))),
                _ => unreachable!(),
            },
            ResponseBodyProj::Stream { stream } => stream.poll_next(cx).map_err(From::from),
        }
    }
}

impl<B> From<Bytes> for ResponseBody<B> {
    fn from(bytes: Bytes) -> Self {
        Self::Bytes { bytes }
    }
}

impl From<StreamBody> for ResponseBody {
    fn from(stream: StreamBody) -> Self {
        Self::Stream { stream }
    }
}
