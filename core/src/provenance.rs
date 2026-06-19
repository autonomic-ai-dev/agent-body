use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainProvenance {
    pub context_id: String,
    pub route_confidence: f64,
    pub skills_used: Vec<String>,
    pub agents_loaded: Vec<String>,
}

impl BrainProvenance {
    pub fn new(context_id: String, route_confidence: f64) -> Self {
        Self {
            context_id,
            route_confidence,
            skills_used: Vec::new(),
            agents_loaded: Vec::new(),
        }
    }
}
