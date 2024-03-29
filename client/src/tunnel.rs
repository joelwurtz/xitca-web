use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use std::sync::Mutex;

use futures_core::stream::Stream;
use futures_sink::Sink;

use super::{
    http::{Method, Version},
    request::RequestBuilder,
};

/// new type of [RequestBuilder] with extended functionality for tunnel handling.
pub struct TunnelRequest<'a, M> {
    pub(crate) req: RequestBuilder<'a>,
    _marker: PhantomData<M>,
}

/// new type of [RequestBuilder] with extended functionality for tunnel handling.

impl<'a, M> Deref for TunnelRequest<'a, M> {
    type Target = RequestBuilder<'a>;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

impl<'a, M> DerefMut for TunnelRequest<'a, M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.req
    }
}

impl<'a, M> TunnelRequest<'a, M> {
    pub(super) fn new(req: RequestBuilder<'a>) -> Self {
        Self {
            req,
            _marker: PhantomData,
        }
    }

    /// Set HTTP method of this request.
    pub fn method(mut self, method: Method) -> Self {
        self.req = self.req.method(method);
        self
    }

    #[doc(hidden)]
    /// Set HTTP version of this request.
    ///
    /// By default request's HTTP version depends on network stream
    pub fn version(mut self, version: Version) -> Self {
        self.req = self.req.version(version);
        self
    }

    /// Set timeout of this request.
    ///
    /// The value passed would override global [ClientBuilder::set_request_timeout].
    ///
    /// [ClientBuilder::set_request_timeout]: crate::ClientBuilder::set_request_timeout
    pub fn timeout(mut self, dur: Duration) -> Self {
        self.req = self.req.timeout(dur);
        self
    }
}

/// sender part of tunneled connection.
/// [Sink] trait is used to asynchronously send message.
pub struct TunnelSink<'a, I>(&'a Tunnel<I>);

impl<M, I> Sink<M> for TunnelSink<'_, I>
where
    I: Sink<M> + Unpin,
{
    type Error = I::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <I as Sink<M>>::poll_ready(Pin::new(&mut *self.get_mut().0.inner.lock().unwrap()), cx)
    }

    fn start_send(self: Pin<&mut Self>, item: M) -> Result<(), Self::Error> {
        Pin::new(&mut *self.get_mut().0.inner.lock().unwrap()).start_send(item)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <I as Sink<M>>::poll_flush(Pin::new(&mut *self.get_mut().0.inner.lock().unwrap()), cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <I as Sink<M>>::poll_close(Pin::new(&mut *self.get_mut().0.inner.lock().unwrap()), cx)
    }
}

/// sender part of tunnel connection.
/// [Stream] trait is used to asynchronously receive message.
pub struct TunnelStream<'a, I>(&'a Tunnel<I>);

impl<I> Stream for TunnelStream<'_, I>
where
    I: Stream + Unpin,
{
    type Item = I::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut *self.get_mut().0.inner.lock().unwrap()).poll_next(cx)
    }
}

/// A unified tunnel that can be used as both sender/receiver.
///
/// * This type can not do concurrent message handling which means send always block receive
/// and vice versa.
pub struct Tunnel<I> {
    pub(crate) inner: Mutex<I>,
}

impl<I> Tunnel<I>
where
    I: Unpin,
{
    /// Split into a sink and reader pair that can be used for concurrent read/write
    /// message to tunnel connection.
    #[inline]
    pub fn split(&self) -> (TunnelSink<'_, I>, TunnelStream<'_, I>) {
        (TunnelSink(self), TunnelStream(self))
    }

    /// leak tunnel from connection reuse pool and underlying connection will not be pushed back to
    /// pool when tunnel is dropped.
    ///
    /// this API does not leak memory
    pub fn leak(self) -> Tunnel<I::Target>
    where
        I: Leak,
        I::Target: Unpin,
    {
        let owned = self.inner.into_inner().unwrap().leak();
        Tunnel::new(owned)
    }

    /// acquire inner tunnel type.
    pub fn into_inner(self) -> I {
        self.inner.into_inner().unwrap()
    }

    pub(crate) fn new(inner: I) -> Self {
        Self {
            inner: Mutex::new(inner),
        }
    }

    fn get_mut_pinned_inner(self: Pin<&mut Self>) -> Pin<&mut I> {
        Pin::new(self.get_mut().inner.get_mut().unwrap())
    }
}

pub trait Leak {
    type Target;

    fn leak(self) -> Self::Target;
}

impl<M, I> Sink<M> for Tunnel<I>
where
    I: Sink<M> + Unpin,
{
    type Error = I::Error;

    #[inline]
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <I as Sink<M>>::poll_ready(self.get_mut_pinned_inner(), cx)
    }

    #[inline]
    fn start_send(self: Pin<&mut Self>, item: M) -> Result<(), Self::Error> {
        self.get_mut_pinned_inner().start_send(item)
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <I as Sink<M>>::poll_flush(self.get_mut_pinned_inner(), cx)
    }

    #[inline]
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <I as Sink<M>>::poll_close(self.get_mut_pinned_inner(), cx)
    }
}

impl<I> Stream for Tunnel<I>
where
    I: Stream + Unpin,
{
    type Item = I::Item;

    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut_pinned_inner().poll_next(cx)
    }
}
