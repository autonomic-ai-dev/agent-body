//! NATS broker credentials, per-organ ACLs, and server config generation.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::global_workspace::broker_dir;

pub const ENV_NATS_USER: &str = "AUTONOMIC_NATS_USER";
pub const ENV_NATS_PASSWORD: &str = "AUTONOMIC_NATS_PASSWORD";
pub const ENV_NATS_TLS: &str = "AUTONOMIC_NATS_TLS";
pub const ENV_NATS_CA: &str = "AUTONOMIC_NATS_CA";
pub const ENV_NATS_CERT: &str = "AUTONOMIC_NATS_CERT";
pub const ENV_NATS_KEY: &str = "AUTONOMIC_NATS_KEY";
pub const ENV_NATS_INSECURE: &str = "AUTONOMIC_NATS_INSECURE";

pub const DLQ_SUBJECT: &str = "system.dlq";

const CREDS_FILE: &str = "creds.json";
const SERVER_CONF: &str = "nats-server.conf";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganNatsAcl {
    pub user: String,
    pub password: String,
    pub publish: Vec<String>,
    pub subscribe: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsCredentialBundle {
    pub version: u32,
    pub port: u16,
    pub http_port: u16,
    pub tls_enabled: bool,
    pub organs: HashMap<String, OrganNatsAcl>,
}

impl NatsCredentialBundle {
    #[must_use]
    pub fn acl_for(&self, organ: &str) -> Option<&OrganNatsAcl> {
        self.organs.get(organ)
    }
}

fn creds_path() -> PathBuf {
    broker_dir().join(CREDS_FILE)
}

pub fn server_config_path() -> PathBuf {
    broker_dir().join(SERVER_CONF)
}

pub fn tls_dir() -> PathBuf {
    broker_dir().join("tls")
}

fn random_password() -> String {
    uuid::Uuid::now_v7().simple().to_string()
}

fn organ_acl(organ: &str, user: &str, password: String) -> OrganNatsAcl {
    let inbox = "_INBOX.>".to_string();
    let js_api = "$JS.API.>".to_string();
    let (publish, subscribe) = match organ {
        "nerves" => (
            vec![
                "autonomic.>".into(),
                "events.>".into(),
                DLQ_SUBJECT.into(),
                js_api.clone(),
                inbox.clone(),
            ],
            vec!["autonomic.>".into(), "events.>".into(), js_api, inbox],
        ),
        "spine" => (
            vec![
                "autonomic.spine.>".into(),
                "autonomic.execute.sandbox".into(),
                DLQ_SUBJECT.into(),
                js_api.clone(),
                inbox.clone(),
            ],
            vec!["autonomic.>".into(), "system.>".into(), js_api, inbox],
        ),
        "immune" => (
            vec![
                "autonomic.execute.result".into(),
                js_api.clone(),
                inbox.clone(),
            ],
            vec!["autonomic.execute.sandbox".into(), js_api, inbox],
        ),
        "muscle" => (
            vec![
                "autonomic.compute.result".into(),
                js_api.clone(),
                inbox.clone(),
            ],
            vec!["autonomic.compute.job".into(), js_api, inbox],
        ),
        "heart" => (
            vec!["events.heart.>".into(), js_api.clone(), inbox.clone()],
            vec!["autonomic.>".into(), "events.>".into(), js_api, inbox],
        ),
        "eyes" => (vec!["events.vision.>".into(), inbox.clone()], vec![inbox]),
        "mouth" => (
            vec!["events.mouth.>".into(), inbox.clone()],
            vec!["autonomic.mouth.>".into(), inbox],
        ),
        "brain" => (
            vec!["events.brain.>".into(), js_api.clone(), inbox.clone()],
            vec!["autonomic.brain.>".into(), js_api, inbox],
        ),
        _ => (vec![inbox.clone()], vec![inbox]),
    };
    OrganNatsAcl {
        user: user.to_string(),
        password,
        publish,
        subscribe,
    }
}

fn default_organs(_port: u16, _tls_enabled: bool) -> HashMap<String, OrganNatsAcl> {
    [
        "nerves", "spine", "immune", "muscle", "heart", "eyes", "mouth", "brain",
    ]
    .into_iter()
    .map(|name| {
        let password = random_password();
        (name.to_string(), organ_acl(name, name, password))
    })
    .collect()
}

/// Load existing credentials or create a fresh bundle (0600 on disk).
pub fn load_or_create_credentials(
    port: u16,
    tls_enabled: bool,
) -> std::io::Result<NatsCredentialBundle> {
    let path = creds_path();
    if path.exists() {
        let raw = fs::read_to_string(&path)?;
        if let Ok(bundle) = serde_json::from_str::<NatsCredentialBundle>(&raw) {
            if bundle.port == port && bundle.tls_enabled == tls_enabled {
                return Ok(bundle);
            }
        }
    }

    let bundle = NatsCredentialBundle {
        version: 1,
        port,
        http_port: 8222,
        tls_enabled,
        organs: default_organs(port, tls_enabled),
    };
    save_credentials(&bundle)?;
    Ok(bundle)
}

pub fn save_credentials(bundle: &NatsCredentialBundle) -> std::io::Result<()> {
    fs::create_dir_all(broker_dir())?;
    let path = creds_path();
    let json = serde_json::to_string_pretty(bundle).map_err(std::io::Error::other)?;
    fs::write(&path, json)?;
    restrict_permissions(&path);
    Ok(())
}

fn restrict_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
}

pub fn nats_insecure_mode() -> bool {
    std::env::var(ENV_NATS_INSECURE)
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Build a NATS URL with optional credentials embedded.
#[must_use]
pub fn build_nats_url(base_url: &str, user: &str, password: &str) -> String {
    if user.is_empty() {
        return base_url.to_string();
    }
    if let Some(rest) = base_url.strip_prefix("nats://") {
        if rest.contains('@') {
            return base_url.to_string();
        }
        return format!("nats://{user}:{password}@{rest}");
    }
    if let Some(rest) = base_url.strip_prefix("tls://") {
        if rest.contains('@') {
            return base_url.to_string();
        }
        return format!("tls://{user}:{password}@{rest}");
    }
    base_url.to_string()
}

/// Resolve connect URL from environment (user/password optional).
#[must_use]
pub fn nats_connect_url_from_env() -> String {
    let base =
        std::env::var("AUTONOMIC_NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string());
    if nats_insecure_mode() {
        return base;
    }
    let user = std::env::var(ENV_NATS_USER)
        .ok()
        .or_else(|| organ_from_env().and_then(|o| load_organ_password(&o).map(|(u, _)| u)))
        .unwrap_or_default();
    let pass = std::env::var(ENV_NATS_PASSWORD)
        .ok()
        .or_else(|| organ_from_env().and_then(|o| load_organ_password(&o).map(|(_, p)| p)))
        .unwrap_or_default();
    if user.is_empty() {
        base
    } else {
        build_nats_url(&base, &user, &pass)
    }
}

fn organ_from_env() -> Option<String> {
    std::env::var("AUTONOMIC_ORGAN").ok()
}

fn load_organ_password(organ: &str) -> Option<(String, String)> {
    let raw = fs::read_to_string(creds_path()).ok()?;
    let bundle: NatsCredentialBundle = serde_json::from_str(&raw).ok()?;
    let acl = bundle.organs.get(organ)?;
    Some((acl.user.clone(), acl.password.clone()))
}

pub fn organ_env_vars(bundle: &NatsCredentialBundle, organ: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(acl) = bundle.acl_for(organ) {
        out.push((ENV_NATS_USER.into(), acl.user.clone()));
        out.push((ENV_NATS_PASSWORD.into(), acl.password.clone()));
    }
    if bundle.tls_enabled {
        let tls = tls_dir();
        out.push((ENV_NATS_TLS.into(), "1".into()));
        out.push((ENV_NATS_CA.into(), tls.join("ca.pem").display().to_string()));
        out.push((
            ENV_NATS_CERT.into(),
            tls.join(format!("client-{organ}.pem"))
                .display()
                .to_string(),
        ));
        out.push((
            ENV_NATS_KEY.into(),
            tls.join(format!("client-{organ}-key.pem"))
                .display()
                .to_string(),
        ));
    }
    out
}

/// Write `nats-server.conf` with JetStream, authorization, and optional TLS.
pub fn write_server_config(bundle: &NatsCredentialBundle) -> std::io::Result<PathBuf> {
    fs::create_dir_all(broker_dir())?;
    let path = server_config_path();
    let mut file = fs::File::create(&path)?;

    writeln!(file, "port: {}", bundle.port)?;
    writeln!(file, "http_port: {}", bundle.http_port)?;
    writeln!(
        file,
        "jetstream {{ store_dir: {:?} }}",
        broker_dir().join("jetstream").display()
    )?;

    if bundle.tls_enabled {
        let tls = tls_dir();
        writeln!(file, "tls {{")?;
        writeln!(file, "  cert_file: {:?}", tls.join("server.pem").display())?;
        writeln!(
            file,
            "  key_file: {:?}",
            tls.join("server-key.pem").display()
        )?;
        writeln!(file, "  ca_file: {:?}", tls.join("ca.pem").display())?;
        writeln!(file, "  verify: true")?;
        writeln!(file, "}}")?;
    }

    if !nats_insecure_mode() {
        writeln!(file, "authorization {{")?;
        writeln!(file, "  users: [")?;
        for acl in bundle.organs.values() {
            writeln!(file, "    {{")?;
            writeln!(file, "      user: {:?}", acl.user)?;
            writeln!(file, "      password: {:?}", acl.password)?;
            writeln!(file, "      permissions: {{")?;
            write!(file, "        publish: [")?;
            for (i, s) in acl.publish.iter().enumerate() {
                if i > 0 {
                    write!(file, ", ")?;
                }
                write!(file, "{s:?}")?;
            }
            writeln!(file, "]")?;
            write!(file, "        subscribe: [")?;
            for (i, s) in acl.subscribe.iter().enumerate() {
                if i > 0 {
                    write!(file, ", ")?;
                }
                write!(file, "{s:?}")?;
            }
            writeln!(file, "]")?;
            writeln!(file, "      }}")?;
            writeln!(file, "    }}")?;
        }
        writeln!(file, "  ]")?;
        writeln!(file, "}}")?;
    }

    restrict_permissions(&path);
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_url_injects_credentials() {
        let url = build_nats_url("nats://127.0.0.1:4222", "spine", "secret");
        assert_eq!(url, "nats://spine:secret@127.0.0.1:4222");
    }

    #[test]
    fn spine_acl_can_publish_dlq() {
        let acl = organ_acl("spine", "spine", "x".into());
        assert!(acl.publish.contains(&DLQ_SUBJECT.to_string()));
    }

    #[test]
    fn eyes_cannot_subscribe_workflow_subjects() {
        let acl = organ_acl("eyes", "eyes", "x".into());
        assert!(!acl.subscribe.iter().any(|s| s.contains("workflow")));
        assert!(
            !acl.subscribe
                .iter()
                .any(|s| s.contains("autonomic.execute"))
        );
    }
}
