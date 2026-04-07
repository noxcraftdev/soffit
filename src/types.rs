use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::theme::{BarStyle, PaletteRole};

/// A theme value that is either a semantic palette role or a raw ANSI 256 index.
/// Serializes as an integer for `Custom` and as a string role name for `Role`.
#[derive(Debug, Clone, PartialEq)]
pub enum ThemeValue {
    Role(PaletteRole),
    Custom(u8),
}

impl Serialize for ThemeValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            ThemeValue::Role(r) => serializer.serialize_str(r.name()),
            ThemeValue::Custom(n) => serializer.serialize_u8(*n),
        }
    }
}

impl<'de> Deserialize<'de> for ThemeValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ThemeValueVisitor;
        impl<'de> serde::de::Visitor<'de> for ThemeValueVisitor {
            type Value = ThemeValue;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "a u8 color index or a palette role name string")
            }
            fn visit_u8<E: serde::de::Error>(self, v: u8) -> Result<ThemeValue, E> {
                Ok(ThemeValue::Custom(v))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<ThemeValue, E> {
                Ok(ThemeValue::Custom(v as u8))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<ThemeValue, E> {
                Ok(ThemeValue::Custom(v as u8))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<ThemeValue, E> {
                PaletteRole::from_name(v)
                    .map(ThemeValue::Role)
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
        deserializer.deserialize_any(ThemeValueVisitor)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct WidgetConfig {
    #[serde(default)]
    pub compact: bool,
    #[serde(default)]
    pub components: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<HashMap<String, ThemeValue>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icons: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_style: Option<BarStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<HashMap<String, Value>>,
}

impl WidgetConfig {
    pub fn has_appearance_overrides(&self) -> bool {
        self.theme.is_some() || self.icons.is_some() || self.bar_style.is_some()
    }

    pub fn has_overrides(&self) -> bool {
        self.has_appearance_overrides() || self.settings.is_some()
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
