//! Customized client compatible with the customized broker.
use std::{fmt::Debug, io::Write, time::Duration};

use serde::{Deserialize, Serialize};

use crate::hush::Secret;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Options {
    pub certificate: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Secret,
}

pub struct Client {
    client: paho_mqtt::AsyncClient,
    _cert: tempfile::NamedTempFile,
}

impl Client {
    pub async fn new(options: &Options) -> anyhow::Result<Self> {
        let create_opts = paho_mqtt::CreateOptionsBuilder::new_v3()
            .server_uri(format!("ssl://{}:{}", options.host, options.port))
            .client_id(&options.username)
            .finalize();
        let client = paho_mqtt::AsyncClient::new(create_opts)?;

        let mut cert = tempfile::NamedTempFile::new()?;
        writeln!(cert, "-----BEGIN CERTIFICATE-----")?;
        writeln!(cert, "{}", options.certificate)?;
        writeln!(cert, "-----END CERTIFICATE-----")?;
        cert.flush()?;

        let ssl_opts = paho_mqtt::SslOptionsBuilder::new()
            .ssl_version(paho_mqtt::SslVersion::Tls_1_2)
            .trust_store(&cert)?
            .finalize();

        let conn_opts = paho_mqtt::ConnectOptionsBuilder::new_v3()
            .clean_session(false)
            .keep_alive_interval(Duration::from_secs(30))
            .ssl_options(ssl_opts)
            .user_name(&options.username)
            .password(options.password.revealed())
            .finalize();
        client.connect(conn_opts).await?;

        Ok(Self {
            client,
            _cert: cert,
        })
    }

    pub fn get_mut(&mut self) -> &mut paho_mqtt::AsyncClient {
        &mut self.client
    }
}
