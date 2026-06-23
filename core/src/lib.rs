pub mod context;
pub mod error;
pub mod execution;
pub mod github_release;
pub mod global_workspace;
pub mod ui;
pub mod nats;
pub mod nats_auth;
#[cfg(feature = "nats")]
pub mod nats_client;
pub mod organ_config;
pub mod provenance;

pub use context::{
    ContextBundle, RouteLimits, ScoredItem, TaskKind, WorkflowTrigger, WorkflowTriggerKind,
};
pub use error::CoreError;
pub use execution::ExecutionId;
pub use global_workspace::{
    autonomic_root, broker_dir, config_path, default_state_db, ensure_dirs, executions_dir,
    legacy_config_path, memory_dir, memory_logs_dir, organ_state_dir, spine_logs_dir,
};
pub use nats::{
    ComputeJob, ComputeResult, ExecuteResult, STREAM_NAME, STREAM_SUBJECT_WILDCARD, SandboxExecute,
    StateTransitionEvent, default_ack_wait, default_duplicate_window, default_nats_url,
};
pub use nats_auth::{
    DLQ_SUBJECT, ENV_NATS_INSECURE, ENV_NATS_PASSWORD, ENV_NATS_TLS, ENV_NATS_USER,
    NatsCredentialBundle, OrganNatsAcl, build_nats_url, load_or_create_credentials,
    nats_connect_url_from_env, nats_insecure_mode, organ_env_vars, save_credentials,
    server_config_path, tls_dir, write_server_config,
};
#[cfg(feature = "nats")]
pub use nats_client::{connect_nats, ping_nats};
pub use provenance::BrainProvenance;
