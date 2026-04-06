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

    install_desktop_entry()?;

    Ok(())
}

fn install_desktop_entry() -> Result<()> {
    let home = dirs::home_dir().context("home directory not found")?;
    let soffit_bin = which_soffit().unwrap_or_else(|| "soffit".into());

    let icon_dir = home.join(".local/share/icons");
    std::fs::create_dir_all(&icon_dir).ok();
    let icon_dest = icon_dir.join("soffit.png");
    if !icon_dest.exists() {
        std::fs::write(&icon_dest, include_bytes!("../assets/icon.png")).ok();
    }

    let apps_dir = home.join(".local/share/applications");
    std::fs::create_dir_all(&apps_dir).ok();
    let desktop_path = apps_dir.join("soffit.desktop");
    let content = format!(
        "[Desktop Entry]\nName=Soffit\nComment=Statusline editor for Claude Code\nExec={} edit\nIcon={}\nType=Application\nCategories=Development;\nStartupWMClass=soffit\n",
        soffit_bin.display(),
        icon_dest.display(),
    );
    std::fs::write(&desktop_path, content).ok();

    Ok(())
}

fn which_soffit() -> Option<PathBuf> {
    std::process::Command::new("which")
        .arg("soffit")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string()))
}

fn claude_settings_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("home directory not found")?;
    Ok(home.join(".claude").join("settings.json"))
}

/// Merges `statusLine` into the settings file at the top level.
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
        .get("statusLine")
        .and_then(|s| s.get("command"))
        .and_then(|c| c.as_str());

    if current_command == Some("soffit render") {
        return Ok(true);
    }

    root["statusLine"] =
        serde_json::json!({ "type": "command", "command": "soffit render", "padding": 0 });

    let serialized = serde_json::to_string_pretty(&root).context("serializing settings.json")?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("creating .claude directory")?;
    }

    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &serialized).context("writing temp settings file")?;
    std::fs::rename(&tmp, path).context("renaming temp settings file")?;

    Ok(false)
}
