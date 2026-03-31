use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct WidgetConfig {
    #[serde(default)]
    pub compact: bool,
    #[serde(default)]
    pub components: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct StdinData {
    pub session_id: Option<String>,
    pub version: Option<String>,
    pub model: Option<ModelInfo>,
    pub context_window: Option<ContextWindow>,
    pub cost: Option<CostInfo>,
    pub workspace: Option<WorkspaceInfo>,
    pub vim: Option<VimInfo>,
    pub agent: Option<AgentInfo>,
    pub rate_limits: Option<RateLimits>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ModelInfo {
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ContextWindow {
    pub used_percentage: Option<f64>,
    pub context_window_size: Option<u64>,
    pub current_usage: Option<CurrentUsage>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CurrentUsage {
    pub input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CostInfo {
    pub total_duration_ms: Option<u64>,
    pub total_cost_usd: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WorkspaceInfo {
    pub current_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct VimInfo {
    pub mode: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AgentInfo {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RateLimits {
    pub five_hour: Option<RateLimit>,
    pub seven_day: Option<RateLimit>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RateLimit {
    pub used_percentage: Option<f64>,
    pub resets_at: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub session_id: String,
    pub model: String,
    pub context_pct: u32,
    pub cwd: String,
    pub updated_at: u64,
}

#[derive(Debug)]
pub struct InsightCounts {
    pub red: u32,
    pub orange: u32,
    pub green: u32,
    pub pending_actions: u32,
}

impl InsightCounts {
    pub fn from_json(items: &[Value]) -> Self {
        let mut red = 0u32;
        let mut orange = 0u32;
        let mut green = 0u32;
        let mut pending = 0u32;

        for item in items {
            let acted = item
                .get("acted_at")
                .is_some_and(|v| v.as_str().is_some_and(|s| !s.is_empty()));

            if acted {
                continue;
            }

            let surfaced = item
                .get("surfaced_at")
                .is_some_and(|v| v.as_str().is_some_and(|s| !s.is_empty()));

            if !surfaced {
                match item.get("urgency").and_then(|v| v.as_str()).unwrap_or("") {
                    "red" => red += 1,
                    "orange" => orange += 1,
                    "green" => green += 1,
                    _ => {}
                }
            } else {
                let action = item.get("action").and_then(|v| v.as_str()).unwrap_or("");
                let is_actionable = !action.is_empty()
                    && !action.to_lowercase().contains("no further action needed")
                    && !action.to_lowercase().contains("for awareness");
                if is_actionable {
                    pending += 1;
                }
            }
        }

        Self {
            red,
            orange,
            green,
            pending_actions: pending,
        }
    }
}
