use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteLimits {
    pub agents: usize,
    pub memory: usize,
    pub rules: usize,
    pub skills: usize,
}

impl Default for RouteLimits {
    fn default() -> Self {
        Self {
            agents: 2,
            memory: 5,
            rules: 5,
            skills: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredItem {
    pub score: f64,
    pub content: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBundle {
    pub rules: Vec<ScoredItem>,
    pub skills: Vec<ScoredItem>,
    pub agents: Vec<ScoredItem>,
    pub bash_allowed_patterns: Vec<String>,
    pub bash_blocked_patterns: Vec<String>,
    pub provenance: Option<crate::provenance::BrainProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowTriggerKind {
    Rule(String),
    Skill(String),
    Agent(String),
    Memory(String, String),
    TriggerWorkflow(WorkflowTrigger),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTrigger {
    pub workflow_path: String,
    pub workflow_name: String,
    pub context_payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskKind {
    Unknown,
    Feature,
    Bugfix,
    Refactor,
    Review,
    Security,
    Database,
    Test,
}
