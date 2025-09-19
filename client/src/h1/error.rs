use std::io;
use xitca_http::BodyError;
use xitca_http::h1::proto::error::ProtoError;

#[derive(Debug)]
pub enum UnexpectedStateError {
    RemainingData,
    ConnectionClosed,
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Proto(ProtoError),
    UnexpectedState(UnexpectedStateError),
    Body(BodyError),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<ProtoError> for Error {
    fn from(e: ProtoError) -> Self {
        Self::Proto(e)
    }
}

impl From<UnexpectedStateError> for Error {
    fn from(e: UnexpectedStateError) -> Self {
        Self::UnexpectedState(e)
    }
}

impl From<BodyError> for Error {
    fn from(e: BodyError) -> Self {
        Self::Body(e)
    }
}
