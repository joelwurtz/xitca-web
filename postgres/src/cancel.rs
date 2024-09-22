//! module for canceling ongoing queries

use postgres_protocol::message::frontend;
use xitca_io::bytes::BytesMut;

use super::{error::Error, session::Session};

impl Session {
    /// Attempts to cancel the in-progress query on the connection associated with Self.
    ///
    /// The server provides no information about whether a cancellation attempt was successful or not. An error will
    /// only be returned if the client was unable to connect to the database.
    ///
    /// Cancellation is inherently racy. There is no guarantee that the cancellation request will reach the server
    /// before the query terminates normally, or that the connection associated with this token is still active.
    pub async fn query_cancel(self) -> Result<(), Error> {
        let Session { id, key, info } = self;
        let (_tx, mut drv) = super::driver::connect_info(info).await?;
        let mut buf = BytesMut::new();
        frontend::cancel_request(id, key, &mut buf);
        drv.send(buf).await
    }
}