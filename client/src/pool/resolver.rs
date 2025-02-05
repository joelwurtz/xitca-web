use std::net::ToSocketAddrs;
use xitca_http::http::{Uri, Version};
use crate::{
    error::Error,
    service::{Service, ServiceDyn},
};
use crate::error::InvalidUri;
use crate::pool::endpoint::Endpoint;

pub type ResolverService =
    Box<dyn for<'r> ServiceDyn<(&'r Uri, Version), Response = Vec<Endpoint>, Error = Error> + Send + Sync>;

pub(crate) fn base_resolver<B>() -> ResolverService {
    struct DefaultResolver;

    impl<'r> Service<(&'r Uri, Version)> for DefaultResolver {
        type Response = Vec<Endpoint>;
        type Error = Error;

        async fn call(&self, (uri, version): (&'r Uri, Version)) -> Result<Self::Response, Self::Error> {
            let (scheme, host) = match (uri.scheme_str(), uri.host()) {
                (_, None) => return Err(Error::InvalidUri(InvalidUri::MissingHost)),
                (None, _) => return Err(Error::InvalidUri(InvalidUri::MissingScheme)),
                (Some(scheme), Some(host)) => (scheme, host),
            };

            if scheme == "unix" {
                let path_and_query = uri.path_and_query().ok_or(Error::InvalidUri(InvalidUri::MissingPathQuery))?;

                let path = format!(
                    "/{}{}",
                    host,
                    path_and_query.as_str()
                );

                return Ok(vec![Endpoint::Unix(path)]);
            }

            let port = uri.port_u16().unwrap_or_else(|| match scheme {
                "http" | "ws" => 80,
                "https" | "wss" => 443,
                _ => 0,
            });

            let is_secure = match scheme {
                "http" | "ws" => false,
                "https" | "wss" => true,
                _ => port == 443,
            };

            let host_for_resolve = host.to_string();
            let addrs = tokio::task::spawn_blocking(move || (host_for_resolve, port).to_socket_addrs())
                .await
                .unwrap()?;

            let mut endpoints = Vec::new();

            for addr in addrs {
                if is_secure {
                    endpoints.push(Endpoint::Secure(addr, host.to_string(), version));
                } else {
                    endpoints.push(Endpoint::Address(addr, version));
                }
            }

            Ok(endpoints)
        }
    }

    Box::new(DefaultResolver)
}
