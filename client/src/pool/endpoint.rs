use std::net::SocketAddr;
use std::time::Instant;
use xitca_http::http::Version;

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum Endpoint {
    Secure(SocketAddr, String, Version),
    Address(SocketAddr, Version),
    Unix(String),
}

impl Endpoint {
    pub fn version(&self) -> Version {
        match self {
            Self::Secure(_, _, version) => *version,
            Self::Address(_, version) => {
                match *version {
                    Version::HTTP_2 => Version::HTTP_2,
                    Version::HTTP_3 => Version::HTTP_3,
                    _ => Version::HTTP_11,
                }
            },
            Self::Unix(_) => Version::HTTP_11,
        }
    }
}

pub enum EndpointState {
    Existing(usize),
    Connecting(usize),
    NotExisting,
    Error(Instant),
}

/*
How a request should be sent to the server .

1. Resolution to a list of Endpoints

This process will use a list of pre determined endpoints from the user if set on the request, or use a dns resolver
to get a list of endpoint

An endpoint is a combination of an address and a maximum http version that can be used to communicate with the server.

The address of an endpoint can be

 - a secure address for https (required for http2 and http3), which is a socket address (ip + port) associated with a sni name
 - a normal address for http (only for http1), which is a socket address (ip + port)
 - a unix address for http (only for http1), which is a path to a unix socket

2. Determining a state for each endpoint

 - Existing : the endpoint is already connected to the server with X current requests processing on it
 - Connecting : the endpoint is currently trying to connect to the server with X current requests waiting for it
 - NotExisting : the endpoint is not connected to the server
 - Error : the endpoint is in error state, since a specific date

3. Choosing the endpoint to use

 - This is where a load balancer can be used to choose the best endpoint to use given the current state of each endpoint

A default strategy could be to choose the endpoint with the least amount of requests processing on it

4. Getting the connection

Given the endpoint choose, we trying to acquire a connection from the pool :

 - If the connection is already established and can be used (either shared, or exclusive with no one using it), we can use it to send the request
 - If the connection is not established, we need to establish it before sending the request

The pool can be :

 * Either a shared pool : (Protocol + SocketAddress) -> ConnectionShared
 * Or an exclusive pool : (Protocol + SocketAddress) -> ConnectionExclusive
 * Or a combination of both
 * Or a NoPool: creating a new connection for each request

The connection established can be exclusive (http1) or shared (http2, http3)

5. Etablishing the connection (Optional)

For max version http1, http2 and http3 (and depending on features enabled), we create a new connection to the server and use it to send the request
If pool allow to add this connection, we add the connection to the pool

For max version http3, and if feature http2 is enabled, we may try to etablish a connection to both http3 and http2 and use the one that succeed first

Once the connection is etablished we can send the request to the server

Once we drop the connection, we should readd it to the pool if the pool allow to add it

 * http2 or http3 can always be added to the pool (even if exclusive)
 * http1 can only be added to the pool if it is exclusive (if not we drop it)


Connection wrapping will be

 -> PoolConnection(EndpointState(Endpoint), Connection)
*/
