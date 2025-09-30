#[cfg(not(feature = "openssl"))]
use tokio_postgres::NoTls;

use crate::error::Error;

/// Builds a OpenSSL connector.
#[cfg(feature = "openssl")]
pub fn tls_connector() -> Result<TlsConnector, Error> {
    use openssl_probe::ProbeResult;
    use std::sync::LazyLock;

    const PROBE_RESULT: LazyLock<ProbeResult> = LazyLock::new(openssl_probe::probe);

    let mut builder = openssl::ssl::SslConnector::builder(openssl::ssl::SslMethod::tls())
        .expect("Failed to create SSL connector builder");

    if let Some(cert_file) = &PROBE_RESULT.cert_file {
        builder.load_verify_locations(Some(cert_file), None)?;
    }
    if let Some(cert_dir) = &PROBE_RESULT.cert_dir {
        builder.load_verify_locations(None, Some(cert_dir))?;
    }

    Ok(postgres_openssl::MakeTlsConnector::new(builder.build()))
}

#[cfg(feature = "openssl")]
pub type TlsConnector = postgres_openssl::MakeTlsConnector;

#[cfg(not(feature = "openssl"))]
pub fn tls_connector() -> Result<TlsConnector, Error> {
    unimplemented!("only openssl is supported")
}

#[cfg(not(feature = "openssl"))]
pub type TlsConnector = NoTls;
