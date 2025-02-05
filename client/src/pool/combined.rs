// use std::net::SocketAddr;
// use std::time::Duration;
// use xitca_http::http::Version;
// use crate::connection::{ConnectionExclusive, ConnectionKey, ConnectionShared};
// use crate::{Connector, Service};
// use crate::error::Error;
// use crate::pool::{exclusive, shared};
// use crate::pool::endpoint::Endpoint;
//
// pub enum Connection {
//     Exclusive(exclusive::Conn<ConnectionKey, ConnectionExclusive>),
//     Shared(shared::Conn<ConnectionKey, ConnectionShared>),
// }
//
// enum Spawner<'a> {
//     Exclusive(exclusive::Spawner<'a, ConnectionKey, ConnectionExclusive>),
//     Shared(shared::Spawner<'a, ConnectionKey, ConnectionShared>),
// }
//
// pub struct PoolService {
//     pub(crate) exclusive_pool: exclusive::Pool<ConnectionKey, ConnectionExclusive>,
//     pub(crate) shared_pool: shared::Pool<ConnectionKey, ConnectionShared>,
//     pub(crate) connect_timeout: Duration,
//     pub(crate) tls_connect_timeout: Duration,
//     pub(crate) local_addr: Option<SocketAddr>,
//     pub(crate) connector: Connector,
// }
//
// impl Service<Endpoint> for PoolService
// {
//     type Response = (Version, Connection);
//     type Error = Error;
//
//     async fn call(&self, endpoint: Endpoint) -> Result<Self::Response, Self::Error> {
//         let spawner = match endpoint.max_http_version {
//             Version::HTTP_2 | Version::HTTP_3 => match self.shared_pool.acquire(&endpoint).await {
//                 shared::AcquireOutput::Conn(conn) => {
//                     return Ok((endpoint.max_http_version, Connection::Shared(conn)));
//                 }
//                 shared::AcquireOutput::Spawner(spawner) => {
//                     Spawner::Shared(spawner)
//                 }
//             }
//             version => match self.exclusive_pool.acquire(&endpoint).await {
//                 exclusive::AcquireOutput::Conn(conn) => {
//                     return Ok((version, Connection::Exclusive(conn)));
//                 }
//                 exclusive::AcquireOutput::Spawner(spawner) => {
//                     Spawner::Exclusive(spawner)
//                 }
//             }
//         };
//
//         unreachable!("");
//
//         // let mut version = endpoint.max_http_version;
//         //
//         // match
//         //
//         // #[cfg(feature = "http3")]
//         // if endpoint.max_http_version == Version::HTTP_3 {
//         //     let mut timer = Box::pin(tokio::time::sleep(self.connect_timeout));
//         //
//         //     if let Ok(Ok(conn)) = crate::h3::proto::connect(
//         //         &client.h3_client,
//         //         connect.addrs(),
//         //         connect.hostname(),
//         //     )
//         //         .timeout(timer.as_mut())
//         //         .await
//         //     {
//         //         _spawner.spawned(conn.into());
//         //     } else {
//         //         #[cfg(feature = "http2")]
//         //         {
//         //             version = Version::HTTP_2;
//         //         }
//         //
//         //         #[cfg(not(feature = "http2"))]
//         //         {
//         //             version = Version::HTTP_11;
//         //         }
//         //     }
//         // }
//
//
//     }
// }