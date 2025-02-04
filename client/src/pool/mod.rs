#![allow(dead_code)]

use std::future::Future;
use std::hash::Hash;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;
use tokio::net::TcpSocket;
use tokio::time::{Instant, Sleep};
use xitca_http::http::Version;
use xitca_io::net::TcpStream;
use crate::{pool, Connect, Connector, Service};
use crate::connection::{ConnectionExclusive, ConnectionKey, ConnectionShared};
use crate::error::{Error, ResolveError, TimeoutError};
use crate::pool::endpoint::Endpoint;
use crate::timeout::Timeout;
use crate::uri::Uri;

// pool for http/1 connections. connection is uniquely owned and ownership is exchanged between
// pool and caller.
pub(crate) mod exclusive;

// pool for http/2 and http/3 connections. connection is shared owned and ownership is reference
// counted between pool and caller.
pub(crate) mod shared;

pub(crate) mod endpoint;

pub enum Connection {
    Exclusive(exclusive::Conn<ConnectionKey, ConnectionExclusive>),
    Shared(shared::Conn<ConnectionKey, ConnectionShared>),
}

enum Spawner<'a> {
    Exclusive(exclusive::Spawner<'a, ConnectionKey, ConnectionExclusive>),
    Shared(shared::Spawner<'a, ConnectionKey, ConnectionShared>),
}

pub struct PoolService {
    pub(crate) exclusive_pool: exclusive::Pool<ConnectionKey, ConnectionExclusive>,
    pub(crate) shared_pool: shared::Pool<ConnectionKey, ConnectionShared>,
    pub(crate) connect_timeout: Duration,
    pub(crate) tls_connect_timeout: Duration,
    pub(crate) local_addr: Option<SocketAddr>,
    pub(crate) connector: Connector,
}

impl Service<Endpoint> for PoolService
{
    type Response = (Version, Connection);
    type Error = Error;

    async fn call(&self, endpoint: Endpoint) -> Result<Self::Response, Self::Error> {
        let spawner = match endpoint.max_http_version {
            Version::HTTP_2 | Version::HTTP_3 => match self.shared_pool.acquire(&endpoint).await {
                shared::AcquireOutput::Conn(conn) => {
                    return Ok((endpoint.max_http_version, Connection::Shared(conn)));
                }
                shared::AcquireOutput::Spawner(spawner) => {
                    Spawner::Shared(spawner)
                }
            }
            version => match self.exclusive_pool.acquire(&endpoint).await {
                exclusive::AcquireOutput::Conn(conn) => {
                    return Ok((version, Connection::Exclusive(conn)));
                }
                exclusive::AcquireOutput::Spawner(spawner) => {
                    Spawner::Exclusive(spawner)
                }
            }
        };

        unreachable!("");

        // let mut version = endpoint.max_http_version;
        //
        // match
        //
        // #[cfg(feature = "http3")]
        // if endpoint.max_http_version == Version::HTTP_3 {
        //     let mut timer = Box::pin(tokio::time::sleep(self.connect_timeout));
        //
        //     if let Ok(Ok(conn)) = crate::h3::proto::connect(
        //         &client.h3_client,
        //         connect.addrs(),
        //         connect.hostname(),
        //     )
        //         .timeout(timer.as_mut())
        //         .await
        //     {
        //         _spawner.spawned(conn.into());
        //     } else {
        //         #[cfg(feature = "http2")]
        //         {
        //             version = Version::HTTP_2;
        //         }
        //
        //         #[cfg(not(feature = "http2"))]
        //         {
        //             version = Version::HTTP_11;
        //         }
        //     }
        // }


    }
}


impl PoolService {
    // make exclusive connection that can be inserted into exclusive connection pool.
    // an expected http version for connection is received and a final http version determined
    // by server side alpn protocol would be returned.
    // when the returned version is HTTP_2 the exclusive connection can be upgraded to shared
    // connection for http2.
    pub(crate) async fn make_exclusive(
        &self,
        connect: &mut Connect<'_>,
        timer: &mut Pin<Box<Sleep>>,
        expected_version: Version,
    ) -> Result<(ConnectionExclusive, Version), Error> {
        match connect.uri {
            Uri::Tcp(_) | Uri::Tls(_) => {
                let conn = self.make_tcp(connect, timer).await?;

                if matches!(connect.uri, Uri::Tcp(_)) {
                    return Ok((conn, expected_version));
                }

                timer
                    .as_mut()
                    .reset(Instant::now() + self.tls_connect_timeout);

                let (conn, version) = self
                    .connector
                    .call((connect.hostname(), conn))
                    .timeout(timer.as_mut())
                    .await
                    .map_err(|_| TimeoutError::TlsHandshake)??;

                Ok((conn, version))
            }
            Uri::Unix(_) => self
                .make_unix(connect, timer)
                .await
                .map(|conn| (conn, expected_version)),
        }
    }

    async fn make_tcp(
        &self,
        connect: &mut Connect<'_>,
        timer: &mut Pin<Box<Sleep>>,
    ) -> Result<ConnectionExclusive, Error> {
        timer
            .as_mut()
            .reset(Instant::now() + self.connect_timeout);

        let stream = self
            .make_tcp_inner(connect)
            .timeout(timer.as_mut())
            .await
            .map_err(|_| TimeoutError::Connect)??;

        // TODO: make nodelay configurable?
        let _ = stream.set_nodelay(true);

        Ok(Box::new(stream))
    }

    async fn make_tcp_inner(&self, connect: &Connect<'_>) -> Result<TcpStream, Error> {
        let mut iter = connect.addrs();

        let mut addr = iter.next().ok_or_else(|| ResolveError::new(connect.hostname()))?;

        // try to connect with all addresses resolved by dns resolver.
        // return the last error when all are fail to be connected.
        loop {
            match self.maybe_connect_with_local_addr(addr).await {
                Ok(stream) => return Ok(stream),
                Err(e) => match iter.next() {
                    Some(a) => addr = a,
                    None => return Err(e),
                },
            }
        }
    }

    async fn maybe_connect_with_local_addr(&self, addr: SocketAddr) -> Result<TcpStream, Error> {
        match self.local_addr {
            Some(local_addr) => {
                let socket = match local_addr {
                    SocketAddr::V4(_) => {
                        let socket = TcpSocket::new_v4()?;
                        socket.bind(local_addr)?;
                        socket
                    }
                    SocketAddr::V6(_) => {
                        let socket = TcpSocket::new_v6()?;
                        socket.bind(local_addr)?;
                        socket
                    }
                };
                let stream = socket.connect(addr).await?;
                Ok(TcpStream::from(stream))
            }
            None => TcpStream::connect(addr).await.map_err(Into::into),
        }
    }

    async fn make_unix(
        &self,
        _connect: &Connect<'_>,
        timer: &mut Pin<Box<Sleep>>,
    ) -> Result<ConnectionExclusive, Error> {
        timer
            .as_mut()
            .reset(Instant::now() + self.connect_timeout);

        #[cfg(unix)]
        {
            let path = format!(
                "/{}{}",
                _connect.uri.authority().unwrap().as_str(),
                _connect.uri.path_and_query().unwrap().as_str()
            );

            let stream = xitca_io::net::UnixStream::connect(path)
                .timeout(timer.as_mut())
                .await
                .map_err(|_| TimeoutError::Connect)??;

            Ok(Box::new(stream))
        }

        #[cfg(not(unix))]
        {
            unimplemented!("only unix supports unix domain socket")
        }
    }
}
