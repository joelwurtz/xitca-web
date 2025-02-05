#![allow(dead_code)]

use std::collections::HashMap;
use crate::connection::{ConnectionExclusive, ConnectionKey, ConnectionShared};
use crate::pool::endpoint::{Endpoint, EndpointState};

// pool for http/1 connections. connection is uniquely owned and ownership is exchanged between
// pool and caller.
pub(crate) mod exclusive;

// pool for http/2 and http/3 connections. connection is shared owned and ownership is reference
// counted between pool and caller.
pub(crate) mod shared;

pub(crate) mod endpoint;
mod resolver;
mod combined;

pub enum Connection {
    Exclusive(exclusive::Conn<ConnectionKey, ConnectionExclusive>),
    Shared(shared::Conn<ConnectionKey, ConnectionShared>),
}

pub trait Pool {
    fn get_endpoints_state(&self, endpoints: Vec<Endpoint>) -> HashMap<Endpoint, EndpointState>;

    fn get_connection(&self, endpoint: Endpoint) -> Connection;
}

pub struct NoPool;

impl Pool for NoPool {
    fn get_endpoints_state(&self, endpoints: Vec<Endpoint>) -> HashMap<Endpoint, EndpointState> {
        endpoints.into_iter().map(|endpoint| (endpoint, EndpointState::NotExisting)).collect()
    }

    fn get_connection(&self, endpoint: Endpoint) {
    }
}