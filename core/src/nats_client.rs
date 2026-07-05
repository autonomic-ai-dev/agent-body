//! Shared async-nats connection with auth and optional mTLS from environment.

#[cfg(feature = "nats")]
use std::path::PathBuf;

#[cfg(feature = "nats")]
use crate::nats_auth::{
    ENV_NATS_CA, ENV_NATS_CERT, ENV_NATS_KEY, ENV_NATS_PASSWORD, ENV_NATS_TLS, ENV_NATS_USER,
};

/// Connect to NATS using `AUTONOMIC_NATS_*` environment variables.
#[cfg(feature = "nats")]
pub async fn connect_nats() -> Result<async_nats::Client, async_nats::ConnectError> {
    let url = crate::nats_auth::nats_connect_url_from_env();
    let mut opts = async_nats::ConnectOptions::new();

    if !crate::nats_auth::nats_insecure_mode() {
        let user = std::env::var(ENV_NATS_USER).unwrap_or_default();
        let pass = std::env::var(ENV_NATS_PASSWORD).unwrap_or_default();
        if !user.is_empty() {
            opts = opts.user_and_password(user, pass);
        }
    }

    if std::env::var(ENV_NATS_TLS).ok().as_deref() == Some("1") {
        opts = opts.require_tls(true);
        if let Ok(ca_path) = std::env::var(ENV_NATS_CA) {
            opts = opts.add_root_certificates(PathBuf::from(ca_path));
        }
        if let (Ok(cert_path), Ok(key_path)) =
            (std::env::var(ENV_NATS_CERT), std::env::var(ENV_NATS_KEY))
        {
            opts = opts.add_client_certificate(PathBuf::from(cert_path), PathBuf::from(key_path));
        }
    }

    opts.connect(url).await
}

#[cfg(feature = "nats")]
pub async fn ping_nats() -> bool {
    connect_nats().await.is_ok()
}
