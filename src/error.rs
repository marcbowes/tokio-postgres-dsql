use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("an unknown error has occurred")]
    Unknown,

    #[error("{0} is an invalid arg")]
    InvalidArg(String),

    #[error("{0}")]
    TokioPostgres(#[from] tokio_postgres::Error),

    #[error("unable to generate token: {0}")]
    TokenError(String),

    #[cfg(feature = "openssl")]
    #[error("{0}")]
    TlsConfig(#[from] openssl::error::ErrorStack),
}
