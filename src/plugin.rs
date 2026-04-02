use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::paths;
use crate::widgets;

fn list_plugins_in(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let seen: HashSet<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            if !path.is_file() {
                return None;
            }
            let stem = path.file_stem()?.to_str()?.to_string();
            let ext = path.extension().and_then(|x| x.to_str()).unwrap_or("");
            if !matches!(ext, "sh" | "py" | "") {
                return None;
            }
            // Prevent shadowing built-in widgets
            if widgets::AVAILABLE.contains(&stem.as_str()) {
                return None;
            }
            Some(stem)
        })
        .collect();

    let mut names: Vec<String> = seen.into_iter().collect();
    names.sort();
    names
}

pub fn list_plugins() -> Vec<String> {
    list_plugins_in(&paths::plugins_dir())
}

#[derive(Debug, Clone, PartialEq)]
pub struct PluginMeta {
    pub name: String,
    pub description: String,
    pub components: Vec<String>,
    pub has_compact: bool,
}

fn list_plugin_metas_in(dir: &Path) -> Vec<PluginMeta> {
    list_plugins_in(dir)
        .into_iter()
        .map(|name| {
            let toml_path = dir.join(format!("{name}.toml"));
            if let Ok(raw) = std::fs::read_to_string(&toml_path) {
                if let Ok(table) = raw.parse::<toml::Table>() {
                    return PluginMeta {
                        name: name.clone(),
                        description: table
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Custom plugin")
                            .to_string(),
                        components: table
                            .get("components")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        has_compact: table
                            .get("has_compact")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                    };
                }
            }
            PluginMeta {
                name,
                description: "Custom plugin".to_string(),
                components: vec![],
                has_compact: false,
            }
        })
        .collect()
}

pub fn list_plugin_metas() -> Vec<PluginMeta> {
    list_plugin_metas_in(&paths::plugins_dir())
}

#[derive(Debug, Clone)]
pub struct OwnedColorSlot {
    pub key: String,
    pub theme_field: String,
}

#[derive(Debug, Clone)]
pub struct OwnedIconSlot {
    pub key: String,
    pub default_value: String,
}

#[derive(Debug, Clone)]
pub struct WidgetMeta {
    pub color_slots: Vec<OwnedColorSlot>,
    pub icon_slots: Vec<OwnedIconSlot>,
}

pub fn widget_meta(name: &str) -> Option<WidgetMeta> {
    use crate::edit::widget_reference::widget_ref;
    if let Some(wref) = widget_ref(name) {
        return Some(WidgetMeta {
            color_slots: wref
                .color_slots
                .iter()
                .map(|s| OwnedColorSlot {
                    key: s.key.to_string(),
                    theme_field: s.theme_field.to_string(),
                })
                .collect(),
            icon_slots: wref
                .icon_slots
                .iter()
                .map(|s| OwnedIconSlot {
                    key: s.key.to_string(),
                    default_value: s.default_value.to_string(),
                })
                .collect(),
        });
    }
    let dir = crate::paths::plugins_dir();
    let script_exists = [name, &format!("{name}.sh")[..], &format!("{name}.py")[..]]
        .iter()
        .any(|f| dir.join(f).exists());
    if !script_exists {
        return None;
    }
    let toml_path = dir.join(format!("{name}.toml"));
    let (color_slots, icon_slots) = std::fs::read_to_string(&toml_path)
        .ok()
        .and_then(|raw| raw.parse::<toml::Table>().ok())
        .map(|table| {
            let cs = table
                .get("colors")
                .and_then(|v| v.as_table())
                .map(|colors_table| {
                    colors_table
                        .iter()
                        .filter_map(|(key, val)| {
                            let sub = val.as_table()?;
                            Some(OwnedColorSlot {
                                key: key.clone(),
                                theme_field: sub.get("theme_field")?.as_str()?.to_string(),
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let is = table
                .get("icons")
                .and_then(|v| v.as_table())
                .map(|icons_table| {
                    icons_table
                        .iter()
                        .filter_map(|(key, val)| {
                            let sub = val.as_table()?;
                            Some(OwnedIconSlot {
                                key: key.clone(),
                                default_value: sub
                                    .get("default_value")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            (cs, is)
        })
        .unwrap_or_default();
    Some(WidgetMeta {
        color_slots,
        icon_slots,
    })
}

pub fn run_plugin(name: &str, stdin_json: &str) -> Option<String> {
    let plugin_dir = paths::plugins_dir();
    let candidates = [
        plugin_dir.join(name),
        plugin_dir.join(format!("{name}.sh")),
        plugin_dir.join(format!("{name}.py")),
    ];
    let path = candidates.iter().find(|p| p.exists())?;

    // Guard against path traversal: canonicalize both and verify containment
    let canonical_plugins = plugin_dir.canonicalize().ok()?;
    let canonical_path = path.canonicalize().ok()?;
    if !canonical_path.starts_with(&canonical_plugins) {
        return None;
    }

    let mut child = Command::new(&canonical_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(stdin_json.as_bytes());
    }

    let child_id = child.id();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = tx.send(result);
    });

    let output = match rx.recv_timeout(std::time::Duration::from_millis(200)) {
        Ok(Ok(out)) if out.status.success() => out,
        _ => {
            #[cfg(unix)]
            // SAFETY: child_id is a valid pid from a process we spawned; SIGKILL is safe to send.
            unsafe {
                libc::kill(child_id as i32, libc::SIGKILL);
            }
            return None;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return None;
    }

    // Plugins may embed raw ANSI ESC bytes (0x1B) in JSON output strings via `echo -e`.
    // Raw ESC bytes are illegal in JSON; escape them so serde_json can parse the structure.
    let json_src = if stdout.contains('\x1b') {
        stdout.replace('\x1b', "\\u001b")
    } else {
        stdout.clone()
    };
    // Try to parse as JSON {"output": "..."}, fall back to raw text
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_src) {
        // If plugin returned "parts", compose them in the requested component order
        if let Some(parts) = v.get("parts").and_then(|p| p.as_object()) {
            let requested: Vec<String> = serde_json::from_str::<serde_json::Value>(stdin_json)
                .ok()
                .and_then(|input| {
                    input
                        .get("config")
                        .and_then(|c| c.get("components"))
                        .and_then(|c| c.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                })
                .unwrap_or_default();
            let compact = serde_json::from_str::<serde_json::Value>(stdin_json)
                .ok()
                .and_then(|input| {
                    input
                        .get("config")
                        .and_then(|c| c.get("compact"))
                        .and_then(|c| c.as_bool())
                })
                .unwrap_or(false);

            let order: Vec<&str> = if requested.is_empty() {
                parts.keys().map(|k| k.as_str()).collect()
            } else {
                requested.iter().map(|s| s.as_str()).collect()
            };
            let sep = if compact { " " } else { " | " };
            let composed: Vec<&str> = order
                .iter()
                .filter_map(|k| parts.get(*k).and_then(|v| v.as_str()))
                .collect();
            if composed.is_empty() {
                None
            } else {
                Some(composed.join(sep))
            }
        } else {
            v.get("output")
                .and_then(|o| o.as_str())
                .map(|s| s.to_string())
        }
    } else {
        Some(stdout)
    }
}

fn source_path_in(dir: &Path, name: &str) -> Option<PathBuf> {
    // Try exact name, then .sh, then .py (same order as run_plugin)
    let candidates = [
        dir.join(name),
        dir.join(format!("{name}.sh")),
        dir.join(format!("{name}.py")),
    ];
    candidates.into_iter().find(|p| p.exists())
}

pub fn plugin_source_path(name: &str) -> Option<PathBuf> {
    source_path_in(&paths::plugins_dir(), name)
}

pub fn read_plugin_source(name: &str) -> Option<String> {
    let path = plugin_source_path(name)?;
    std::fs::read_to_string(path).ok()
}

pub fn write_plugin_source(name: &str, content: &str) -> anyhow::Result<()> {
    write_plugin_source_in(&paths::plugins_dir(), name, content)
}

fn write_plugin_source_in(dir: &Path, name: &str, content: &str) -> anyhow::Result<()> {
    let path =
        source_path_in(dir, name).ok_or_else(|| anyhow::anyhow!("plugin not found: {name}"))?;
    std::fs::write(&path, content)?;
    // Preserve executable permission on unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&path, perms)?;
    }
    Ok(())
}

pub fn create_plugin(name: &str, ext: &str, template: &str) -> anyhow::Result<()> {
    create_plugin_in(&paths::plugins_dir(), name, ext, template)
}

fn create_plugin_in(dir: &Path, name: &str, ext: &str, template: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let filename = if ext.is_empty() {
        name.to_string()
    } else {
        format!("{name}.{ext}")
    };
    let path = dir.join(&filename);
    if path.exists() {
        anyhow::bail!("plugin already exists: {filename}");
    }
    std::fs::write(&path, template)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    // Create empty sidecar
    let toml_path = dir.join(format!("{name}.toml"));
    if !toml_path.exists() {
        std::fs::write(
            &toml_path,
            format!("description = \"Custom plugin: {name}\"\n"),
        )?;
    }
    Ok(())
}

pub fn delete_plugin(name: &str) -> anyhow::Result<()> {
    delete_plugin_in(&paths::plugins_dir(), name)
}

pub(crate) fn delete_plugin_in(dir: &Path, name: &str) -> anyhow::Result<()> {
    // Remove all possible script files
    for candidate in [
        dir.join(name),
        dir.join(format!("{name}.sh")),
        dir.join(format!("{name}.py")),
    ] {
        if candidate.exists() {
            std::fs::remove_file(&candidate)?;
        }
    }
    // Remove sidecar
    let toml_path = dir.join(format!("{name}.toml"));
    if toml_path.exists() {
        std::fs::remove_file(&toml_path)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct PluginOutput {
    pub text: String,
    pub components: Vec<String>,
    pub parts: HashMap<String, String>,
}

impl PluginOutput {
    /// Returns the output text, reordered by `requested` component order if parts are available.
    pub fn compose(&self, requested: &[String], compact: bool) -> String {
        if self.parts.is_empty() {
            return self.text.clone();
        }
        let order: Vec<&str> = if requested.is_empty() {
            self.components.iter().map(|s| s.as_str()).collect()
        } else {
            requested.iter().map(|s| s.as_str()).collect()
        };
        let sep = if compact { " " } else { " | " };
        let composed: Vec<&str> = order
            .iter()
            .filter_map(|k| self.parts.get(*k).map(|s| s.as_str()))
            .collect();
        if composed.is_empty() {
            self.text.clone()
        } else {
            composed.join(sep)
        }
    }
}

pub fn run_plugin_full(name: &str, stdin_json: &str) -> Option<PluginOutput> {
    let plugin_dir = paths::plugins_dir();
    let candidates = [
        plugin_dir.join(name),
        plugin_dir.join(format!("{name}.sh")),
        plugin_dir.join(format!("{name}.py")),
    ];
    let path = candidates.iter().find(|p| p.exists())?;

    let canonical_plugins = plugin_dir.canonicalize().ok()?;
    let canonical_path = path.canonicalize().ok()?;
    if !canonical_path.starts_with(&canonical_plugins) {
        return None;
    }

    let mut child = Command::new(&canonical_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(stdin_json.as_bytes());
    }

    let child_id = child.id();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = tx.send(result);
    });

    let output = match rx.recv_timeout(std::time::Duration::from_millis(200)) {
        Ok(Ok(out)) if out.status.success() => out,
        _ => {
            #[cfg(unix)]
            unsafe {
                libc::kill(child_id as i32, libc::SIGKILL);
            }
            return None;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return None;
    }

    let json_src = if stdout.contains('\x1b') {
        stdout.replace('\x1b', "\\u001b")
    } else {
        stdout.clone()
    };
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_src) {
        let text = v
            .get("output")
            .and_then(|o| o.as_str())
            .unwrap_or("")
            .to_string();
        let components = v
            .get("components")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let parts = v
            .get("parts")
            .and_then(|p| p.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        Some(PluginOutput {
            text,
            components,
            parts,
        })
    } else {
        Some(PluginOutput {
            text: stdout,
            components: vec![],
            parts: HashMap::new(),
        })
    }
}

pub fn rename_plugin(old_name: &str, new_name: &str) -> anyhow::Result<()> {
    let dir = paths::plugins_dir();
    if new_name.is_empty() || new_name.contains(' ') || new_name.contains('/') {
        anyhow::bail!("invalid plugin name: {new_name}");
    }
    // Rename script file
    if let Some(old_path) = source_path_in(&dir, old_name) {
        let ext = old_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let new_filename = if ext.is_empty() {
            new_name.to_string()
        } else {
            format!("{new_name}.{ext}")
        };
        let new_path = dir.join(&new_filename);
        if new_path.exists() {
            anyhow::bail!("plugin already exists: {new_filename}");
        }
        std::fs::rename(&old_path, &new_path)?;
    } else {
        anyhow::bail!("plugin not found: {old_name}");
    }
    // Rename sidecar
    let old_toml = dir.join(format!("{old_name}.toml"));
    let new_toml = dir.join(format!("{new_name}.toml"));
    if old_toml.exists() {
        std::fs::rename(&old_toml, &new_toml)?;
    }
    Ok(())
}

pub fn import_plugin(source_path: &Path) -> anyhow::Result<String> {
    let dir = paths::plugins_dir();
    std::fs::create_dir_all(&dir)?;
    let file_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("invalid source path"))?;
    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("invalid source path"))?;
    let dest = dir.join(file_name);
    if dest.exists() {
        anyhow::bail!("plugin already exists: {file_name}");
    }
    std::fs::copy(source_path, &dest)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
    }
    // Create sidecar if missing
    let toml_path = dir.join(format!("{stem}.toml"));
    if !toml_path.exists() {
        std::fs::write(&toml_path, format!("description = \"Imported: {stem}\"\n"))?;
    }
    Ok(stem.to_string())
}

pub fn mock_stdin_json() -> String {
    serde_json::json!({
        "data": {
            "session_id": "test-preview",
            "version": "1.2.16",
            "model": {"display_name": "claude-sonnet-4-6"},
            "context_window": {
                "used_percentage": 42.0,
                "context_window_size": 200000,
                "current_usage": {"input_tokens": 84000}
            },
            "cost": {"total_duration_ms": 4830000, "total_cost_usd": 0.42},
            "workspace": {"current_dir": "/home/user/project"},
            "vim": {"mode": "NORMAL"},
            "agent": {"name": "worker-1"},
            "rate_limits": {
                "five_hour": {"used_percentage": 60.0},
                "seven_day": {"used_percentage": 75.0}
            }
        },
        "config": {
            "compact": false,
            "components": []
        }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{
        create_plugin_in, delete_plugin_in, list_plugin_metas_in, list_plugins_in, mock_stdin_json,
        source_path_in, write_plugin_source_in, PluginMeta,
    };

    fn make_file(dir: &TempDir, name: &str) {
        fs::write(dir.path().join(name), "").unwrap();
    }

    fn write_plugin_meta_in(
        dir: &std::path::Path,
        name: &str,
        meta: &PluginMeta,
    ) -> anyhow::Result<()> {
        let toml_path = dir.join(format!("{name}.toml"));
        let mut table = toml::Table::new();
        table.insert(
            "description".to_string(),
            toml::Value::String(meta.description.clone()),
        );
        if !meta.components.is_empty() {
            table.insert(
                "components".to_string(),
                toml::Value::try_from(meta.components.clone())?,
            );
        }
        if meta.has_compact {
            table.insert("has_compact".to_string(), toml::Value::Boolean(true));
        }
        std::fs::write(&toml_path, toml::to_string_pretty(&table)?)?;
        Ok(())
    }

    // ---- list_plugins_in ----------------------------------------------------

    #[test]
    fn discovers_sh_py_and_bare_executables() {
        let dir = TempDir::new().unwrap();
        make_file(&dir, "myplugin.sh");
        make_file(&dir, "another.py");
        make_file(&dir, "bare_exec");

        let mut plugins = list_plugins_in(dir.path());
        plugins.sort();

        assert_eq!(plugins, vec!["another", "bare_exec", "myplugin"]);
    }

    #[test]
    fn toml_sidecar_alone_is_not_a_plugin() {
        let dir = TempDir::new().unwrap();
        make_file(&dir, "myplugin.toml");

        let plugins = list_plugins_in(dir.path());
        assert!(plugins.is_empty());
    }

    #[test]
    fn deduplicates_stem_with_and_without_extension() {
        let dir = TempDir::new().unwrap();
        make_file(&dir, "foo");
        make_file(&dir, "foo.sh");

        let plugins = list_plugins_in(dir.path());
        assert_eq!(plugins, vec!["foo"]);
    }

    #[test]
    fn filters_out_built_in_widgets() {
        let dir = TempDir::new().unwrap();
        // "git" is a known built-in from widgets::AVAILABLE
        make_file(&dir, "git.sh");
        make_file(&dir, "custom.sh");

        let plugins = list_plugins_in(dir.path());
        assert_eq!(plugins, vec!["custom"]);
    }

    #[test]
    fn returns_empty_for_nonexistent_dir() {
        let plugins = list_plugins_in(std::path::Path::new("/nonexistent/path/that/never/exists"));
        assert!(plugins.is_empty());
    }

    // ---- list_plugin_metas_in -----------------------------------------------

    #[test]
    fn plugin_without_sidecar_gets_defaults() {
        let dir = TempDir::new().unwrap();
        make_file(&dir, "simple.sh");

        let metas = list_plugin_metas_in(dir.path());
        assert_eq!(metas.len(), 1);
        let m = &metas[0];
        assert_eq!(m.name, "simple");
        assert_eq!(m.description, "Custom plugin");
        assert!(m.components.is_empty());
        assert!(!m.has_compact);
    }

    #[test]
    fn plugin_with_sidecar_is_parsed() {
        let dir = TempDir::new().unwrap();
        make_file(&dir, "mywidget.sh");
        fs::write(
            dir.path().join("mywidget.toml"),
            r#"description = "My widget"
components = ["a", "b"]
has_compact = true
"#,
        )
        .unwrap();

        let metas = list_plugin_metas_in(dir.path());
        assert_eq!(metas.len(), 1);
        let m = &metas[0];
        assert_eq!(m.name, "mywidget");
        assert_eq!(m.description, "My widget");
        assert_eq!(m.components, vec!["a", "b"]);
        assert!(m.has_compact);
    }

    #[test]
    fn malformed_toml_falls_back_to_defaults() {
        let dir = TempDir::new().unwrap();
        make_file(&dir, "broken.sh");
        fs::write(dir.path().join("broken.toml"), "not valid toml ][[[").unwrap();

        let metas = list_plugin_metas_in(dir.path());
        assert_eq!(metas.len(), 1);
        let m = &metas[0];
        assert_eq!(m.description, "Custom plugin");
        assert!(m.components.is_empty());
        assert!(!m.has_compact);
    }

    // ---- source_path_in -----------------------------------------------------

    #[test]
    fn source_path_prefers_exact_then_sh_then_py() {
        let dir = TempDir::new().unwrap();
        make_file(&dir, "myplugin.py");
        assert!(source_path_in(dir.path(), "myplugin").is_some());

        make_file(&dir, "myplugin.sh");
        let p = source_path_in(dir.path(), "myplugin").unwrap();
        // .sh wins over .py because it is tried second (after exact name)
        assert_eq!(p.extension().and_then(|e| e.to_str()), Some("sh"));
    }

    #[test]
    fn source_path_returns_none_for_missing_plugin() {
        let dir = TempDir::new().unwrap();
        assert!(source_path_in(dir.path(), "ghost").is_none());
    }

    // ---- write_plugin_source_in / create_plugin_in / delete_plugin_in -------

    #[test]
    fn write_and_read_plugin_source() {
        let dir = TempDir::new().unwrap();
        make_file(&dir, "myplugin.sh");

        write_plugin_source_in(dir.path(), "myplugin", "#!/bin/sh\necho hi").unwrap();

        let content = fs::read_to_string(dir.path().join("myplugin.sh")).unwrap();
        assert_eq!(content, "#!/bin/sh\necho hi");
    }

    #[test]
    fn write_plugin_source_errors_on_missing_plugin() {
        let dir = TempDir::new().unwrap();
        assert!(write_plugin_source_in(dir.path(), "ghost", "content").is_err());
    }

    #[test]
    fn create_plugin_writes_script_and_sidecar() {
        let dir = TempDir::new().unwrap();
        create_plugin_in(dir.path(), "myplugin", "sh", "#!/bin/sh\necho hello").unwrap();

        assert!(dir.path().join("myplugin.sh").exists());
        assert!(dir.path().join("myplugin.toml").exists());
        let toml = fs::read_to_string(dir.path().join("myplugin.toml")).unwrap();
        assert!(toml.contains("myplugin"));
    }

    #[test]
    fn create_plugin_rejects_duplicate() {
        let dir = TempDir::new().unwrap();
        create_plugin_in(dir.path(), "dup", "sh", "").unwrap();
        assert!(create_plugin_in(dir.path(), "dup", "sh", "").is_err());
    }

    #[test]
    fn delete_plugin_removes_script_and_sidecar() {
        let dir = TempDir::new().unwrap();
        make_file(&dir, "gone.sh");
        make_file(&dir, "gone.toml");

        delete_plugin_in(dir.path(), "gone").unwrap();

        assert!(!dir.path().join("gone.sh").exists());
        assert!(!dir.path().join("gone.toml").exists());
    }

    #[test]
    fn delete_plugin_noop_on_missing() {
        let dir = TempDir::new().unwrap();
        // Must not error when nothing exists
        delete_plugin_in(dir.path(), "ghost").unwrap();
    }

    // ---- write_plugin_meta_in -----------------------------------------------

    #[test]
    fn write_plugin_meta_roundtrips() {
        let dir = TempDir::new().unwrap();
        let meta = PluginMeta {
            name: "myplugin".to_string(),
            description: "does stuff".to_string(),
            components: vec!["a".to_string(), "b".to_string()],
            has_compact: true,
        };
        write_plugin_meta_in(dir.path(), "myplugin", &meta).unwrap();

        let metas = list_plugin_metas_in(dir.path());
        // No script file exists, so list returns empty — verify TOML is parseable directly
        let raw = fs::read_to_string(dir.path().join("myplugin.toml")).unwrap();
        let table: toml::Table = raw.parse().unwrap();
        assert_eq!(table["description"].as_str().unwrap(), "does stuff");
        assert!(table["has_compact"].as_bool().unwrap());
        // list_plugin_metas_in won't include it (no script), but the TOML is valid
        assert!(metas.is_empty());
    }

    // ---- mock_stdin_json ----------------------------------------------------

    #[test]
    fn mock_stdin_json_is_valid_json() {
        let s = mock_stdin_json();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["data"]["session_id"], "test-preview");
        assert!(v["config"]["components"].is_array());
    }
}
