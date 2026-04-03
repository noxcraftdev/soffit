use anyhow::{Context, Result};
use std::path::PathBuf;

pub fn run() -> Result<()> {
    let settings_path = claude_settings_path()?;
    let already_set = update_settings(&settings_path)?;

    if already_set {
        println!("soffit: statusLine already set to 'soffit render' — nothing to do");
    } else {
        println!("soffit: wrote statusLine to {}", settings_path.display());
        println!("soffit: restart Claude Code to activate");
    }

    Ok(())
}

fn claude_settings_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("home directory not found")?;
    Ok(home.join(".claude").join("settings.json"))
}

/// Merges `env.statusLine.command = "soffit render"` into the settings file.
/// Returns `true` if the file was already correctly configured (no write needed).
fn update_settings(path: &PathBuf) -> Result<bool> {
    let raw = if path.exists() {
        std::fs::read_to_string(path).context("reading settings.json")?
    } else {
        "{}".to_string()
    };

    let mut root: serde_json::Value =
        serde_json::from_str(&raw).context("parsing settings.json")?;

    let current_command = root
        .get("env")
        .and_then(|e| e.get("statusLine"))
        .and_then(|s| s.get("command"))
        .and_then(|c| c.as_str());

    if current_command == Some("soffit render") {
        return Ok(true);
    }

    if !root.get("env").map(|v| v.is_object()).unwrap_or(false) {
        root["env"] = serde_json::json!({});
    }

    root["env"]["statusLine"] = serde_json::json!({ "command": "soffit render" });

    let serialized = serde_json::to_string_pretty(&root).context("serializing settings.json")?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("creating .claude directory")?;
    }

    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &serialized).context("writing temp settings file")?;
    std::fs::rename(&tmp, path).context("renaming temp settings file")?;

    Ok(false)
}
