pub mod agents_compose;
pub mod cli;
pub mod config_migrate;
pub mod context;
pub mod ecosystem_config;
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
pub mod workspace_sync;
pub mod workspace_update;
#[cfg(feature = "wasm")]
pub mod wasm_engine;

pub use context::{
    ContextBundle, RouteLimits, ScoredItem, TaskKind, WorkflowTrigger, WorkflowTriggerKind,
};
pub use error::CoreError;
pub use execution::ExecutionId;
pub use agents_compose::{
    compose_agents_md, default_fragment, install_host_agents_md_links, scaffold_agents_dir,
    write_fragment, BASE_AGENT_MODE, FRAGMENT_ORDER,
};
pub use config_migrate::{
    migrate_brain_yaml_if_needed, migrate_spine_legacy_if_needed, run_legacy_migrations,
};
pub use ecosystem_config::{
    agents_config, effective_sync, effective_update, ensure_default_ecosystem_sections,
    read_organ_section_raw, update_enabled_for_organ, write_organ_section_raw, AgentsConfig,
    GitSyncConfig, SyncConfig, UpdateConfig,
};
pub use global_workspace::{
    agents_dir, agents_md_path, autonomic_root, broker_dir, config_path, default_state_db,
    ensure_dirs, executions_dir, legacy_brain_home, legacy_config_path, legacy_spine_config_dir,
    memory_dir, memory_logs_dir, organ_state_dir, spine_config_dir, spine_logs_dir,
    workspace_gitignore_path,
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
pub use workspace_sync::{
    git_init, git_pull, git_push, git_status, write_default_gitignore, DEFAULT_GITIGNORE,
};
pub use workspace_update::{organ_alias_for_binary, should_update_binary, should_update_organ};
pub use provenance::BrainProvenance;
#[cfg(feature = "wasm")]
pub use wasm_engine::{default_fuel_limit, WasmEngine};
