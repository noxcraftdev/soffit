use anyhow::{bail, Result};
use std::process::Command;

const REPO: &str = "noxcraftdev/soffit";

pub fn run() -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    println!("soffit {current}");
    println!("Checking for updates...");

    let output = Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "10",
            &format!("https://api.github.com/repos/{REPO}/releases/latest"),
        ])
        .output()?;

    if !output.status.success() {
        bail!("failed to check for updates");
    }

    let v: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let tag = v
        .get("tag_name")
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow::anyhow!("no release found"))?;
    let latest = tag.strip_prefix('v').unwrap_or(tag);

    if latest == current {
        println!("Already up to date.");
        return Ok(());
    }

    println!("New version available: {latest}");

    let exe = std::env::current_exe()?;
    let exe_str = exe.to_string_lossy();

    if exe_str.contains("/.cargo/bin/") {
        println!("Updating via cargo...");
        let status = Command::new("cargo")
            .args(["install", "soffit", "--force"])
            .status()?;
        if !status.success() {
            bail!("cargo install failed");
        }
    } else {
        println!("Downloading soffit {latest}...");
        let target = detect_target()?;
        let url =
            format!("https://github.com/{REPO}/releases/download/v{latest}/soffit-{target}.tar.gz");

        let tmp = std::env::temp_dir().join("soffit-update");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp)?;

        let tarball = tmp.join("soffit.tar.gz");
        let status = Command::new("curl")
            .args(["-fsSL", &url, "-o"])
            .arg(&tarball)
            .status()?;
        if !status.success() {
            bail!("download failed — no pre-built binary for {target}");
        }

        let status = Command::new("tar")
            .args(["xzf"])
            .arg(&tarball)
            .arg("-C")
            .arg(&tmp)
            .status()?;
        if !status.success() {
            bail!("failed to extract archive");
        }

        let new_binary = tmp.join("soffit");
        if !new_binary.exists() {
            bail!("binary not found in archive");
        }

        let exe_tmp = exe.with_extension("tmp");
        std::fs::copy(&new_binary, &exe_tmp)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&exe_tmp, std::fs::Permissions::from_mode(0o755))?;
        }
        std::fs::rename(&exe_tmp, &exe)?;

        let _ = std::fs::remove_dir_all(&tmp);
    }

    println!("Updated to soffit {latest}");
    Ok(())
}

fn detect_target() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu".to_string()),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin".to_string()),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin".to_string()),
        _ => anyhow::bail!(
            "unsupported platform: {os}/{arch} — install from source with: cargo install soffit"
        ),
    }
}
