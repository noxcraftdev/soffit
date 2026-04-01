use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::theme::{BarStyle, PaletteRole};

/// A color value that is either a semantic palette role or a raw ANSI 256 index.
/// Serializes as an integer for `Custom` and as a string role name for `Role`.
#[derive(Debug, Clone, PartialEq)]
pub enum ColorValue {
    Role(PaletteRole),
    Custom(u8),
}

impl Serialize for ColorValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            ColorValue::Role(r) => serializer.serialize_str(r.name()),
            ColorValue::Custom(n) => serializer.serialize_u8(*n),
        }
    }
}

impl<'de> Deserialize<'de> for ColorValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ColorValueVisitor;
        impl<'de> serde::de::Visitor<'de> for ColorValueVisitor {
            type Value = ColorValue;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "a u8 color index or a palette role name string")
            }
            fn visit_u8<E: serde::de::Error>(self, v: u8) -> Result<ColorValue, E> {
                Ok(ColorValue::Custom(v))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<ColorValue, E> {
                Ok(ColorValue::Custom(v as u8))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<ColorValue, E> {
                Ok(ColorValue::Custom(v as u8))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<ColorValue, E> {
                PaletteRole::from_name(v)
                    .map(ColorValue::Role)
                    .ok_or_else(|| {
                        E::unknown_variant(
                            v,
                            &[
                                "primary", "accent", "success", "warning", "danger", "muted",
                                "subtle",
                            ],
                        )
                    })
            }
        }
        deserializer.deserialize_any(ColorValueVisitor)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct WidgetConfig {
    #[serde(default)]
    pub compact: bool,
    #[serde(default)]
    pub components: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colors: Option<HashMap<String, ColorValue>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icons: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_style: Option<BarStyle>,
}

impl WidgetConfig {
    pub fn has_appearance_overrides(&self) -> bool {
        self.colors.is_some() || self.icons.is_some() || self.bar_style.is_some()
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct ModelInfo {
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct ContextWindow {
    pub used_percentage: Option<f64>,
    pub context_window_size: Option<u64>,
    pub current_usage: Option<CurrentUsage>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct CurrentUsage {
    pub input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct CostInfo {
    pub total_duration_ms: Option<u64>,
    pub total_cost_usd: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct WorkspaceInfo {
    pub current_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct VimInfo {
    pub mode: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct AgentInfo {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct RateLimits {
    pub five_hour: Option<RateLimit>,
    pub seven_day: Option<RateLimit>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
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
