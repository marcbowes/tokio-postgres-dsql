use aws_config::{BehaviorVersion, SdkConfig};
use aws_sdk_dsql::auth_token::AuthTokenGenerator;
use tls::TlsConnector;
use tokio::task::JoinHandle;
use tokio_postgres::{Client, Config, config::Host};

mod error;
pub use error::Error;

mod tls;

#[derive(Clone)]
pub struct Opts {
    sdk_config: SdkConfig,
    config: Config,
}

impl Opts {
    /// Create a new `Opts` with a custom AWS SDK config
    pub fn new(conninfo: &str, sdk_config: SdkConfig) -> Result<Opts, Error> {
        let config = conninfo.parse::<Config>()?;
        Ok(Opts { sdk_config, config })
    }

    /// Create a new `Opts` by loading AWS config from the environment
    pub async fn from_conninfo(conninfo: &str) -> Result<Opts, Error> {
        let config = conninfo.parse::<Config>()?;
        let sdk_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        Ok(Opts { sdk_config, config })
    }

    /// Open a single connection that will automatically reconnect if
    /// disconnected. This is suitable for lightweight environments that don't
    /// need a connection pool.
    pub async fn connect_one(&self) -> Result<SingleConnection, Error> {
        let tls = tls::tls_connector()?;
        let mut c = SingleConnection {
            opts: self.clone(),
            tls,
            client: None,
            connection: None,
        };
        c.reconnect().await?;
        Ok(c)
    }

    /// Create a single connection without establishing an initial connection.
    /// The connection will be established lazily on the first call to `borrow()`.
    pub async fn lazy_one(&self) -> Result<SingleConnection, Error> {
        let tls = tls::tls_connector()?;
        let c = SingleConnection {
            opts: self.clone(),
            tls,
            client: None,
            connection: None,
        };
        Ok(c)
    }
}

/// A single connection to Aurora DSQL. If disconnected, the connection will
/// automatically reopen.
pub struct SingleConnection {
    opts: Opts,
    tls: TlsConnector,
    client: Option<Client>,
    connection: Option<JoinHandle<Result<(), tokio_postgres::Error>>>,
}

impl SingleConnection {
    /// Returns a connected [`tokio_postgres::Client`]. If disconnected, attempt
    /// to reconnect (once).
    pub async fn borrow(&mut self) -> Result<&mut Client, Error> {
        // First check if we need to reconnect (without borrowing client)
        let needs_reconnect = match (&self.client, &self.connection) {
            (Some(_), Some(connection)) => connection.is_finished(),
            _ => true,
        };

        if needs_reconnect {
            self.reconnect().await?;
        }

        // Now we can safely return the client reference
        self.client.as_mut().ok_or(Error::Unknown)
    }

    /// Close any existing connection, then open a new one.
    pub async fn reconnect(&mut self) -> Result<(), Error> {
        let mut config = self.opts.config.clone();

        let host = match config.get_hosts() {
            [Host::Tcp(name)] => name,
            _ => return Err(Error::InvalidArg("host".to_string())),
        };

        let user = match config.get_user() {
            Some(user) => user,
            _ => return Err(Error::InvalidArg("user".to_string())),
        };

        let region = match self.opts.sdk_config.region() {
            Some(r) => r,
            _ => return Err(Error::InvalidArg("region".to_string())),
        };

        let signer = AuthTokenGenerator::new(
            aws_sdk_dsql::auth_token::Config::builder()
                .hostname(host.clone())
                .region(region.clone())
                .build()
                .expect("args are always valid"),
        );

        let token = match user {
            "admin" => {
                signer
                    .db_connect_admin_auth_token(&self.opts.sdk_config)
                    .await
            }
            _ => signer.db_connect_auth_token(&self.opts.sdk_config).await,
        }
        .map_err(|err| Error::TokenError(err.to_string()))?
        .to_string();
        config.password(token);

        let (client, connection) = config.connect(self.tls.clone()).await?;
        let task = tokio::spawn(connection);

        self.client.replace(client);
        self.connection.replace(task);

        Ok(())
    }
}
