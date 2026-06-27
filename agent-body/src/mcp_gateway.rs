use std::collections::HashMap;
use std::sync::Arc;

use rmcp::model::*;
use rmcp::service::{RoleClient, RunningService};
use rmcp::transport::ConfigureCommandExt;
use rmcp::transport::TokioChildProcess;
use rmcp::{ErrorData, ServiceExt};
use tokio::process::Command;
use tokio::sync::Mutex;

struct OrganRoute {
    binary: &'static str,
    subcommand: &'static str,
}

fn organ_routes() -> HashMap<&'static str, OrganRoute> {
    let mut m = HashMap::new();
    m.insert("heart", OrganRoute { binary: "agent-heart", subcommand: "serve-mcp" });
    m.insert("muscle", OrganRoute { binary: "agent-muscle", subcommand: "serve-mcp" });
    m.insert("eyes", OrganRoute { binary: "agent-eyes", subcommand: "serve-mcp" });
    m.insert("immune", OrganRoute { binary: "agent-immune", subcommand: "serve-mcp" });
    m.insert("mouth", OrganRoute { binary: "agent-mouth", subcommand: "mcp" });
    m.insert("spine", OrganRoute { binary: "agent-spine", subcommand: "mcp-serve" });
    m
}

fn tool_to_organ(tool_name: &str) -> &'static str {
    match tool_name.split('_').next() {
        Some("heart") => "heart",
        Some("muscle") => "muscle",
        Some("eyes") => "eyes",
        Some("immune") => "immune",
        Some("mouth") => "mouth",
        Some("spine") => "spine",
        _ => "heart",
    }
}

fn tool_definitions() -> Vec<Tool> {
    let obj = || Arc::new(serde_json::Map::new());
    vec![
        Tool::new("heart_gc", "Run memory garbage collection on agent-brain", obj()),
        Tool::new("heart_budget_status", "Return current token/cost consumption across the session", obj()),
        Tool::new("heart_memory_distill", "Summarize related memory facts into higher-level concepts", obj()),
        Tool::new("heart_status", "Show agent-heart daemon status", obj()),
        Tool::new("muscle_execute_bash", "Execute a shell script/command via tokio::process::Command", obj()),
        Tool::new("muscle_execute_python", "Run a Python snippet in an isolated interpreter", obj()),
        Tool::new("eyes_describe_dom", "Parse raw HTML into a token-efficient JSON layout of interactive elements", obj()),
        Tool::new("eyes_diff_screenshots", "Capture a localhost URL screenshot and pixel-diff against a baseline", obj()),
        Tool::new("eyes_vlm_caption", "Run a local Candle LLaVA model to caption an image", obj()),
        Tool::new("immune_scan_manifest", "Fuzz Cargo.toml or package.json against OSV.dev for known CVEs", obj()),
        Tool::new("immune_sandbox_run", "Execute an untrusted script in a Firecracker microVM", obj()),
        Tool::new("immune_lint_ast", "Run tree-sitter AST linting against security policy", obj()),
        Tool::new("mouth_validate_ast", "Pass a bash command through tree-sitter AST to check policy violations", obj()),
        Tool::new("mouth_request_approval", "Fire a Slack/Discord webhook for human approval before destructive ops", obj()),
        Tool::new("spine_submit_workflow", "Submit a YAML workflow DAG definition for execution and return a workflow ID", obj()),
        Tool::new("spine_check_status", "Check the status of a workflow execution by ID", obj()),
        Tool::new("spine_list_workflows", "List recent workflow executions with their IDs, names, statuses, and timestamps", obj()),
    ]
}

async fn connect_organ(route: &OrganRoute) -> Result<RunningService<RoleClient, ()>, ErrorData> {
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
        ErrorData::internal_error(format!("failed to spawn {}: {e}", route.binary), None)
    })?;

    ().serve(transport).await.map_err(|e| {
        ErrorData::internal_error(format!("MCP connect to {} failed: {e}", route.binary), None)
    })
}

pub struct McpGateway {
    sessions: Mutex<HashMap<&'static str, RunningService<RoleClient, ()>>>,
}

impl McpGateway {
    pub fn new() -> Self {
        McpGateway {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub async fn run() -> anyhow::Result<()> {
        let server = McpGateway::new();
        let service = rmcp::serve_server(server, rmcp::transport::io::stdio()).await?;
        service.waiting().await?;
        Ok(())
    }
}

impl rmcp::ServerHandler for McpGateway {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_instructions(
            "Autonomic AI MCP Gateway — unified entry point for all organ MCP tools. Routes calls to organ binaries (agent-heart, agent-muscle, agent-eyes, agent-immune, agent-mouth, agent-spine).",
        )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: tool_definitions(),
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let organ_key = tool_to_organ(&request.name);
        let routes = organ_routes();
        let route = routes.get(organ_key).ok_or_else(|| {
            ErrorData::internal_error(format!("no route for organ '{organ_key}'"), None)
        })?;

        let mut sessions = self.sessions.lock().await;
        if !sessions.contains_key(organ_key) {
            let service = connect_organ(route).await?;
            sessions.insert(organ_key, service);
        }

        let service = sessions.get(organ_key).ok_or_else(|| {
            ErrorData::internal_error(format!("session vanished for organ '{organ_key}'"), None)
        })?;

        let result = service.call_tool(request).await.map_err(|e| {
            ErrorData::internal_error(format!("tool call failed: {e}"), None)
        })?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_to_organ_routes_by_prefix() {
        assert_eq!(tool_to_organ("heart_gc"), "heart");
        assert_eq!(tool_to_organ("eyes_describe_dom"), "eyes");
        assert_eq!(tool_to_organ("immune_scan_manifest"), "immune");
        assert_eq!(tool_to_organ("muscle_execute_bash"), "muscle");
        assert_eq!(tool_to_organ("mouth_validate_ast"), "mouth");
        assert_eq!(tool_to_organ("spine_submit_workflow"), "spine");
    }

    #[test]
    fn tool_to_organ_unknown_falls_back_to_heart() {
        assert_eq!(tool_to_organ("unknown_tool"), "heart");
    }

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
    fn tool_definitions_includes_all_organs() {
        let tools = tool_definitions();
        let names: Vec<&str> = tools.iter().map(|t| &*t.name).collect();
        assert!(names.contains(&"heart_gc"));
        assert!(names.contains(&"heart_budget_status"));
        assert!(names.contains(&"eyes_describe_dom"));
        assert!(names.contains(&"immune_scan_manifest"));
        assert!(names.contains(&"muscle_execute_bash"));
        assert!(names.contains(&"mouth_validate_ast"));
        assert!(names.contains(&"spine_submit_workflow"));
        assert_eq!(tools.len(), 17);
    }

    #[test]
    fn all_tools_map_to_known_organs() {
        let tools = tool_definitions();
        for tool in &tools {
            let organ = tool_to_organ(&tool.name);
            let routes = organ_routes();
            assert!(
                routes.contains_key(organ),
                "tool {} maps to organ {organ} which has no route",
                tool.name
            );
        }
    }
}
