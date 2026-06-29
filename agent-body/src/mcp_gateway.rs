use std::collections::HashMap;
use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ErrorData as McpError, Implementation,
    ServerCapabilities, ServerInfo,
};
use rmcp::service::{RoleClient, RunningService};
use rmcp::transport::ConfigureCommandExt;
use rmcp::transport::TokioChildProcess;
use rmcp::{tool, tool_handler, tool_router, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::sync::Mutex;

struct OrganRoute {
    binary: &'static str,
    subcommand: &'static str,
}

fn organ_routes() -> HashMap<&'static str, OrganRoute> {
    let mut m = HashMap::new();
    m.insert(
        "heart",
        OrganRoute {
            binary: "agent-heart",
            subcommand: "serve-mcp",
        },
    );
    m.insert(
        "muscle",
        OrganRoute {
            binary: "agent-muscle",
            subcommand: "serve-mcp",
        },
    );
    m.insert(
        "eyes",
        OrganRoute {
            binary: "agent-eyes",
            subcommand: "serve-mcp",
        },
    );
    m.insert(
        "immune",
        OrganRoute {
            binary: "agent-immune",
            subcommand: "serve-mcp",
        },
    );
    m.insert(
        "mouth",
        OrganRoute {
            binary: "agent-mouth",
            subcommand: "mcp",
        },
    );
    m.insert(
        "spine",
        OrganRoute {
            binary: "agent-spine",
            subcommand: "mcp-serve",
        },
    );
    m
}

async fn connect_organ(route: &OrganRoute) -> Result<RunningService<RoleClient, ()>, McpError> {
    let mut command = Command::new(route.binary);
    command.arg(route.subcommand);

    if route.binary == "agent-spine" {
        let db = agent_body_core::organ_state_dir("spine").join("state.db");
        if db.exists() {
            command.arg("--db");
            command.arg(db.to_string_lossy().as_ref());
        }
    }

    let transport = TokioChildProcess::new(command.configure(|_| {})).map_err(|e| {
        McpError::internal_error(format!("failed to spawn {}: {e}", route.binary), None)
    })?;

    ().serve(transport).await.map_err(|e| {
        McpError::internal_error(format!("MCP connect to {} failed: {e}", route.binary), None)
    })
}

#[derive(Clone)]
pub struct McpGateway {
    sessions: Arc<Mutex<HashMap<&'static str, RunningService<RoleClient, ()>>>>,
}

impl McpGateway {
    pub fn new() -> Self {
        McpGateway {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn run() -> anyhow::Result<()> {
        let server = McpGateway::new();
        let service = server.serve(rmcp::transport::io::stdio()).await?;
        service.waiting().await?;
        Ok(())
    }

    async fn forward(
        &self,
        organ_key: &'static str,
        tool_name: &'static str,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        let routes = organ_routes();
        let route = routes.get(organ_key).ok_or_else(|| {
            McpError::internal_error(format!("no route for organ '{organ_key}'"), None)
        })?;

        let mut sessions = self.sessions.lock().await;
        if !sessions.contains_key(organ_key) {
            let service = connect_organ(route).await?;
            sessions.insert(organ_key, service);
        }

        let service = sessions.get(organ_key).ok_or_else(|| {
            McpError::internal_error(format!("session vanished for organ '{organ_key}'"), None)
        })?;

        let mut req = CallToolRequestParams::new(tool_name.to_string());
        if let Some(args) = arguments.as_object().cloned() {
            if !args.is_empty() {
                req = req.with_arguments(args);
            }
        }

        service.call_tool(req).await.map_err(|e| {
            McpError::internal_error(format!("tool call failed: {e}"), None)
        })
    }

    async fn forward_params<T: serde::Serialize>(
        &self,
        organ_key: &'static str,
        tool_name: &'static str,
        params: T,
    ) -> Result<CallToolResult, McpError> {
        let arguments = serde_json::to_value(params)
            .map_err(|e| McpError::internal_error(format!("serialize params: {e}"), None))?;
        self.forward(organ_key, tool_name, arguments).await
    }
}

#[derive(Debug, Default, Deserialize, Serialize, JsonSchema)]
struct EmptyParams {}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct HeartGcParams {
    #[serde(default = "default_min_confidence")]
    min_confidence: f64,
    #[serde(default = "default_max_age_days")]
    max_age_days: u64,
}

fn default_min_confidence() -> f64 {
    0.3
}

fn default_max_age_days() -> u64 {
    90
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct HeartDistillParams {
    #[serde(default = "default_threshold")]
    threshold: f64,
    #[serde(default)]
    dry_run: bool,
}

fn default_threshold() -> f64 {
    0.75
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct MuscleExecuteParams {
    command: String,
    #[serde(default)]
    cwd: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct MusclePythonParams {
    code: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct EyesDescribeDomParams {
    html: String,
    #[serde(default = "default_max_elements")]
    max_elements: u32,
}

fn default_max_elements() -> u32 {
    5000
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct EyesDiffScreenshotsParams {
    url: String,
    #[serde(default)]
    baseline: Option<String>,
    #[serde(default = "default_diff_threshold")]
    threshold: f64,
}

fn default_diff_threshold() -> f64 {
    1.0
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct EyesVlmCaptionParams {
    image: String,
    #[serde(default)]
    prompt: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct ImmuneScanManifestParams {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct ImmuneSandboxRunParams {
    #[serde(default)]
    script_path: Option<String>,
    #[serde(default)]
    script_content: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct ImmuneLintAstParams {
    code: String,
    language: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct MouthValidateAstParams {
    command: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct MouthRequestApprovalParams {
    message: String,
    action: String,
    #[serde(default)]
    webhook_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct SpineSubmitWorkflowParams {
    yaml: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct SpineCheckStatusParams {
    workflow_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct SpineListWorkflowsParams {
    #[serde(default = "default_list_limit")]
    limit: u32,
}

fn default_list_limit() -> u32 {
    10
}

#[tool_router]
impl McpGateway {
    #[tool(description = "Run memory garbage collection on agent-brain")]
    async fn heart_gc(
        &self,
        params: Parameters<HeartGcParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward_params("heart", "heart_gc", params.0).await
    }

    #[tool(description = "Return current token/cost consumption across the session")]
    async fn heart_budget_status(
        &self,
        _params: Parameters<EmptyParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward("heart", "heart_budget_status", serde_json::json!({}))
            .await
    }

    #[tool(
        description = "Summarize related memory facts into higher-level concepts via cluster distillation"
    )]
    async fn heart_memory_distill(
        &self,
        params: Parameters<HeartDistillParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward_params("heart", "heart_memory_distill", params.0)
            .await
    }

    #[tool(description = "Show agent-heart daemon status")]
    async fn heart_status(
        &self,
        _params: Parameters<EmptyParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward("heart", "heart_status", serde_json::json!({}))
            .await
    }

    #[tool(description = "Execute a shell script/command via tokio::process::Command")]
    async fn muscle_execute_bash(
        &self,
        params: Parameters<MuscleExecuteParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward_params("muscle", "muscle_execute_bash", params.0)
            .await
    }

    #[tool(description = "Run a Python snippet in an isolated interpreter")]
    async fn muscle_execute_python(
        &self,
        params: Parameters<MusclePythonParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward_params("muscle", "muscle_execute_python", params.0)
            .await
    }

    #[tool(
        description = "Parse raw HTML into a token-efficient JSON layout of interactive elements"
    )]
    async fn eyes_describe_dom(
        &self,
        params: Parameters<EyesDescribeDomParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward_params("eyes", "eyes_describe_dom", params.0)
            .await
    }

    #[tool(description = "Capture a localhost URL screenshot and pixel-diff against a baseline")]
    async fn eyes_diff_screenshots(
        &self,
        params: Parameters<EyesDiffScreenshotsParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward_params("eyes", "eyes_diff_screenshots", params.0)
            .await
    }

    #[tool(description = "Run a local Candle LLaVA model to caption an image")]
    async fn eyes_vlm_caption(
        &self,
        params: Parameters<EyesVlmCaptionParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward_params("eyes", "eyes_vlm_caption", params.0)
            .await
    }

    #[tool(description = "Fuzz Cargo.toml or package.json against OSV.dev for known CVEs")]
    async fn immune_scan_manifest(
        &self,
        params: Parameters<ImmuneScanManifestParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward_params("immune", "immune_scan_manifest", params.0)
            .await
    }

    #[tool(description = "Execute an untrusted script in a sandboxed environment")]
    async fn immune_sandbox_run(
        &self,
        params: Parameters<ImmuneSandboxRunParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward_params("immune", "immune_sandbox_run", params.0)
            .await
    }

    #[tool(description = "Run AST-based security linting against a code snippet")]
    async fn immune_lint_ast(
        &self,
        params: Parameters<ImmuneLintAstParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward_params("immune", "immune_lint_ast", params.0)
            .await
    }

    #[tool(
        description = "Pass a bash command through tree-sitter AST validation to check security policy violations"
    )]
    async fn mouth_validate_ast(
        &self,
        params: Parameters<MouthValidateAstParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward_params("mouth", "mouth_validate_ast", params.0)
            .await
    }

    #[tool(
        description = "Request human approval before destructive operations via Slack/Discord webhook"
    )]
    async fn mouth_request_approval(
        &self,
        params: Parameters<MouthRequestApprovalParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward_params("mouth", "mouth_request_approval", params.0)
            .await
    }

    #[tool(
        description = "Submit a YAML workflow DAG definition for execution and return a workflow ID"
    )]
    async fn spine_submit_workflow(
        &self,
        params: Parameters<SpineSubmitWorkflowParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward_params("spine", "spine_submit_workflow", params.0)
            .await
    }

    #[tool(description = "Check the status of a workflow execution by ID")]
    async fn spine_check_status(
        &self,
        params: Parameters<SpineCheckStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward_params("spine", "spine_check_status", params.0)
            .await
    }

    #[tool(
        description = "List recent workflow executions with their IDs, names, statuses, and timestamps"
    )]
    async fn spine_list_workflows(
        &self,
        params: Parameters<SpineListWorkflowsParams>,
    ) -> Result<CallToolResult, McpError> {
        self.forward_params("spine", "spine_list_workflows", params.0)
            .await
    }
}

#[tool_handler]
impl ServerHandler for McpGateway {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.instructions = Some(
            "Autonomic AI MCP Gateway — unified entry point for all organ MCP tools. \
             Routes calls to organ binaries (agent-heart, agent-muscle, agent-eyes, agent-immune, agent-mouth, agent-spine)."
                .into(),
        );
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        let mut impl_info = Implementation::default();
        impl_info.name = "agent-body".into();
        impl_info.version = env!("CARGO_PKG_VERSION").into();
        info.server_info = impl_info;
        info
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn organ_routes_contains_all_six_organs() {
        let routes = organ_routes();
        assert_eq!(routes.len(), 6);
        assert_eq!(routes.get("heart").unwrap().subcommand, "serve-mcp");
        assert_eq!(routes.get("muscle").unwrap().subcommand, "serve-mcp");
        assert_eq!(routes.get("eyes").unwrap().subcommand, "serve-mcp");
        assert_eq!(routes.get("immune").unwrap().subcommand, "serve-mcp");
        assert_eq!(routes.get("mouth").unwrap().subcommand, "mcp");
        assert_eq!(routes.get("spine").unwrap().subcommand, "mcp-serve");
    }

    #[test]
    fn mcp_gateway_implements_server_handler() {
        fn assert_handler<T: rmcp::ServerHandler>() {}
        assert_handler::<McpGateway>();
    }

    #[test]
    fn muscle_execute_params_require_command() {
        let schema = schemars::schema_for!(MuscleExecuteParams);
        let json = serde_json::to_value(&schema).expect("schema");
        let required = json
            .get("required")
            .and_then(|v| v.as_array())
            .expect("required array");
        assert!(required.iter().any(|v| v == "command"));
    }
}
