//! Shared NATS JetStream subjects and message payloads for the autonomic ecosystem.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Primary JetStream stream capturing all autonomic subjects.
pub const STREAM_NAME: &str = "AUTONOMIC";

/// Wildcard subject space owned by the AUTONOMIC stream.
pub const STREAM_SUBJECT_WILDCARD: &str = "autonomic.>";

/// Default NATS URL from the environment (`AUTONOMIC_NATS_URL`).
pub fn default_nats_url() -> Option<String> {
    std::env::var("AUTONOMIC_NATS_URL").ok()
}

/// Default duplicate-detection window (exactly-once publishing).
pub fn default_duplicate_window() -> Duration {
    Duration::from_secs(120)
}

/// Default time before un-ACKed messages are redelivered.
pub fn default_ack_wait() -> Duration {
    Duration::from_secs(30)
}

pub mod subjects {
    pub const SPINE_STATE: &str = "autonomic.spine.state";
    pub const COMPUTE_JOB: &str = "autonomic.compute.job";
    pub const COMPUTE_RESULT: &str = "autonomic.compute.result";
    pub const EXECUTE_SANDBOX: &str = "autonomic.execute.sandbox";
    pub const EXECUTE_RESULT: &str = "autonomic.execute.result";
}

/// Workflow lifecycle event published by agent-spine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransitionEvent {
    pub msg_id: String,
    pub execution_id: Option<String>,
    pub workflow_name: Option<String>,
    pub event_type: String,
    pub payload: serde_json::Value,
}

/// Remote command job consumed by agent-muscle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeJob {
    pub msg_id: String,
    pub job_id: String,
    pub command: String,
    #[serde(default)]
    pub cwd: Option<String>,
}

/// Result of a compute job, published back to the bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeResult {
    pub msg_id: String,
    pub job_id: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    pub duration_ms: u64,
}

/// Sandbox execution request consumed by agent-immune.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxExecute {
    pub msg_id: String,
    pub job_id: String,
    pub command: String,
    #[serde(default)]
    pub cwd: Option<String>,
}

/// Sandbox execution result published for agent-spine orchestration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResult {
    pub msg_id: String,
    pub job_id: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}
