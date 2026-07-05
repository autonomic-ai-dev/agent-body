//! Graceful degradation ladder when organ health scores drop.

use serde::{Deserialize, Serialize};

/// Runtime capability tier (Kernel V2 Phase 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradationLevel {
    /// All organs at full capability.
    Full,
    /// agent-mouth returns raw JSON instead of SLM-polished prose.
    MouthRawJson,
    /// agent-eyes skips VLM caption; DOM-only perception.
    EyesDomOnly,
    /// Critical mesh failure — MCP tools gated except spine/heart essentials.
    Minimal,
}

impl DegradationLevel {
    pub fn from_mesh_score(score: u8) -> Self {
        match score {
            0..=20 => Self::Minimal,
            21..=40 => Self::EyesDomOnly,
            41..=60 => Self::MouthRawJson,
            _ => Self::Full,
        }
    }

    pub fn allows_organ(&self, organ: &str) -> bool {
        match self {
            Self::Full | Self::MouthRawJson | Self::EyesDomOnly => true,
            Self::Minimal => matches!(organ, "heart" | "spine" | "immune"),
        }
    }

    pub fn mouth_mode(&self) -> &'static str {
        match self {
            Self::Full => "slm",
            Self::MouthRawJson | Self::EyesDomOnly | Self::Minimal => "raw_json",
        }
    }

    pub fn eyes_mode(&self) -> &'static str {
        match self {
            Self::Full | Self::MouthRawJson => "vlm",
            Self::EyesDomOnly | Self::Minimal => "dom_only",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationState {
    pub level: DegradationLevel,
    pub mesh_score: u8,
    pub reason: String,
}

impl DegradationState {
    pub fn from_mesh_score(score: u8) -> Self {
        let level = DegradationLevel::from_mesh_score(score);
        let reason = format!("mesh_score={score} → {:?}", level);
        Self {
            level,
            mesh_score: score,
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder_steps_down_with_score() {
        assert_eq!(
            DegradationLevel::from_mesh_score(90),
            DegradationLevel::Full
        );
        assert_eq!(
            DegradationLevel::from_mesh_score(55),
            DegradationLevel::MouthRawJson
        );
        assert_eq!(
            DegradationLevel::from_mesh_score(35),
            DegradationLevel::EyesDomOnly
        );
        assert_eq!(
            DegradationLevel::from_mesh_score(10),
            DegradationLevel::Minimal
        );
    }
}
