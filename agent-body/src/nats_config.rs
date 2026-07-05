//! Bootstrap NATS credentials, server config, and optional mTLS material.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use rcgen::{CertificateParams, DistinguishedName, DnType, IsCa, KeyPair, SanType};

use agent_body_core::{
    ensure_dirs, load_or_create_credentials, tls_dir, write_server_config, NatsCredentialBundle,
};

const DEFAULT_PORT: u16 = 4222;

pub struct NatsBootstrap {
    pub bundle: NatsCredentialBundle,
    pub config_path: std::path::PathBuf,
}

pub fn tls_enabled_from_env() -> bool {
    std::env::var("AUTONOMIC_NATS_TLS")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Ensure broker credentials, server config, and optional TLS certs exist.
pub fn ensure_nats_security() -> Result<NatsBootstrap> {
    ensure_dirs().map_err(|e| anyhow::anyhow!(e))?;
    let tls = tls_enabled_from_env();
    let bundle = load_or_create_credentials(DEFAULT_PORT, tls).map_err(|e| anyhow::anyhow!(e))?;
    if tls {
        ensure_tls_material(&bundle)?;
    }
    let config_path = write_server_config(&bundle).map_err(|e| anyhow::anyhow!(e))?;
    Ok(NatsBootstrap {
        bundle,
        config_path,
    })
}

fn ensure_tls_material(bundle: &NatsCredentialBundle) -> Result<()> {
    let dir = tls_dir();
    fs::create_dir_all(&dir)?;
    let ca_path = dir.join("ca.pem");
    if ca_path.exists() {
        return Ok(());
    }

    let mut ca_params =
        CertificateParams::new(vec!["autonomic-ca".into()]).context("create CA params")?;
    ca_params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "Autonomic NATS CA");
    ca_params.distinguished_name = dn;
    let ca_key = KeyPair::generate().context("generate CA key")?;
    let ca_cert = ca_params.self_signed(&ca_key).context("sign CA cert")?;
    fs::write(&ca_path, ca_cert.pem()).context("write ca.pem")?;
    fs::write(dir.join("ca-key.pem"), ca_key.serialize_pem()).context("write ca-key.pem")?;

    let mut server_params = CertificateParams::new(vec!["localhost".into(), "127.0.0.1".into()])
        .context("create server params")?;
    server_params
        .subject_alt_names
        .push(SanType::IpAddress(std::net::IpAddr::V4(
            std::net::Ipv4Addr::LOCALHOST,
        )));
    let mut server_dn = DistinguishedName::new();
    server_dn.push(DnType::CommonName, "Autonomic NATS Server");
    server_params.distinguished_name = server_dn;
    let server_key = KeyPair::generate().context("generate server key")?;
    let server_cert = server_params
        .signed_by(&server_key, &ca_cert, &ca_key)
        .context("sign server cert")?;
    fs::write(dir.join("server.pem"), server_cert.pem()).context("write server.pem")?;
    fs::write(dir.join("server-key.pem"), server_key.serialize_pem())
        .context("write server-key.pem")?;

    for organ in bundle.organs.keys() {
        write_client_cert(&dir, organ, &ca_cert, &ca_key)?;
    }

    Ok(())
}

fn write_client_cert(
    dir: &Path,
    organ: &str,
    ca_cert: &rcgen::Certificate,
    ca_key: &KeyPair,
) -> Result<()> {
    let mut params = CertificateParams::new(vec![format!("{organ}.autonomic")])
        .context("create client params")?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, format!("Autonomic {organ}"));
    params.distinguished_name = dn;
    let key = KeyPair::generate().context("generate client key")?;
    let cert = params
        .signed_by(&key, ca_cert, ca_key)
        .context("sign client cert")?;
    fs::write(dir.join(format!("client-{organ}.pem")), cert.pem()).context("write client cert")?;
    fs::write(
        dir.join(format!("client-{organ}-key.pem")),
        key.serialize_pem(),
    )
    .context("write client key")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_env_detection() {
        std::env::set_var("AUTONOMIC_NATS_TLS", "1");
        assert!(tls_enabled_from_env());
        std::env::remove_var("AUTONOMIC_NATS_TLS");
    }
}
