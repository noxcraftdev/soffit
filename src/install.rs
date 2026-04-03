use std::path::Path;

use anyhow::{anyhow, bail};

use crate::{paths, plugin, widgets};

fn curl_fetch(url: &str) -> anyhow::Result<Vec<u8>> {
    crate::http::curl_fetch(url)
}

/// Returns (owner, repo, name_opt).
fn parse_source(s: &str) -> anyhow::Result<(String, String, Option<String>)> {
    let parts: Vec<&str> = s.splitn(3, '/').collect();
    match parts.as_slice() {
        [owner, repo] => Ok((owner.to_string(), repo.to_string(), None)),
        [owner, repo, name] => Ok((owner.to_string(), repo.to_string(), Some(name.to_string()))),
        _ => bail!("invalid source '{s}': expected owner/repo or owner/repo/name"),
    }
}

#[derive(Debug)]
struct RepoFile {
    name: String,
    path: String,
}

fn list_repo_plugins(owner: &str, repo: &str) -> anyhow::Result<Vec<RepoFile>> {
    // Try root first, then /plugins subdirectory.
    let entries =
        try_list_dir_raw(owner, repo, "").or_else(|_| try_list_dir_raw(owner, repo, "plugins"))?;

    let mut files: Vec<RepoFile> = entries
        .iter()
        .filter_map(|item: &serde_json::Value| {
            let name = item.get("name")?.as_str()?.to_string();
            let path = item.get("path")?.as_str()?.to_string();
            let kind = item.get("type")?.as_str().unwrap_or("");
            if kind != "file" {
                return None;
            }
            if !name.ends_with(".sh") && !name.ends_with(".py") {
                return None;
            }
            Some(RepoFile { name, path })
        })
        .collect();

    // If no script files were found at root, check for per-plugin subdirectories.
    if files.is_empty() {
        for item in &entries {
            let kind = item
                .get("type")
                .and_then(|v: &serde_json::Value| v.as_str())
                .unwrap_or("");
            if kind != "dir" {
                continue;
            }
            let dirname = match item
                .get("name")
                .and_then(|v: &serde_json::Value| v.as_str())
            {
                Some(n) => n,
                None => continue,
            };
            let ext = "sh";
            files.push(RepoFile {
                name: format!("{dirname}.{ext}"),
                path: format!("{dirname}/{dirname}.{ext}"),
            });
        }
    }

    Ok(files)
}

fn try_list_dir_raw(
    owner: &str,
    repo: &str,
    subdir: &str,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let url = if subdir.is_empty() {
        format!("https://api.github.com/repos/{owner}/{repo}/contents")
    } else {
        format!("https://api.github.com/repos/{owner}/{repo}/contents/{subdir}")
    };
    let bytes = curl_fetch(&url)?;
    let arr: serde_json::Value = serde_json::from_slice(&bytes)?;
    arr.as_array()
        .ok_or_else(|| anyhow!("expected array from GitHub API"))
        .cloned()
}

pub(crate) fn install_one_in(
    dir: &Path,
    name: &str,
    ext: &str,
    script: &[u8],
    toml_opt: Option<Vec<u8>>,
    force: bool,
) -> anyhow::Result<()> {
    if name.contains('/') || name.contains('\\') || name.starts_with('.') || name.contains('\0') {
        bail!("unsafe plugin name: '{name}'");
    }

    if widgets::AVAILABLE.contains(&name) {
        bail!("'{name}' conflicts with a built-in widget");
    }

    std::fs::create_dir_all(dir)?;

    // Duplicate check.
    let script_path = dir.join(format!("{name}.{ext}"));
    let sh_path = dir.join(format!("{name}.sh"));
    let py_path = dir.join(format!("{name}.py"));
    let bare_path = dir.join(name);

    let already_exists =
        script_path.exists() || sh_path.exists() || py_path.exists() || bare_path.exists();

    if already_exists && !force {
        bail!("plugin '{name}' already installed (use --force to overwrite)");
    }

    if already_exists && force {
        plugin::delete_plugin_in(dir, name)?;
    }

    // Write script.
    std::fs::write(&script_path, script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;
    }

    // Write sidecar — use provided or generate default.
    let toml_path = dir.join(format!("{name}.toml"));
    if let Some(toml_bytes) = toml_opt {
        std::fs::write(&toml_path, toml_bytes)?;
    } else {
        std::fs::write(&toml_path, format!("description = \"Installed: {name}\"\n"))?;
    }

    Ok(())
}

pub(crate) fn raw_url(owner: &str, repo: &str, path: &str) -> String {
    format!("https://raw.githubusercontent.com/{owner}/{repo}/main/{path}")
}

fn fetch_optional(url: &str) -> Option<Vec<u8>> {
    curl_fetch(url).ok().filter(|b| !b.is_empty())
}

pub fn run(source: &str, force: bool) -> anyhow::Result<()> {
    if !source.contains('/') {
        return crate::marketplace::resolve_and_install(source, force);
    }
    let (owner, repo, name_opt) = parse_source(source)?;
    let plugins_dir = paths::plugins_dir();

    match name_opt {
        Some(name) => install_single(&owner, &repo, &name, &plugins_dir, force),
        None => install_all(&owner, &repo, &plugins_dir, force),
    }
}

fn install_single(
    owner: &str,
    repo: &str,
    name: &str,
    dir: &Path,
    force: bool,
) -> anyhow::Result<()> {
    // Try root then per-plugin subdir, .sh before .py.
    let (script, ext, prefix) = if let Some(bytes) =
        fetch_optional(&raw_url(owner, repo, &format!("{name}.sh")))
    {
        (bytes, "sh", "")
    } else if let Some(bytes) = fetch_optional(&raw_url(owner, repo, &format!("{name}/{name}.sh")))
    {
        (bytes, "sh", name)
    } else if let Some(bytes) = fetch_optional(&raw_url(owner, repo, &format!("{name}.py"))) {
        (bytes, "py", "")
    } else if let Some(bytes) = fetch_optional(&raw_url(owner, repo, &format!("{name}/{name}.py")))
    {
        (bytes, "py", name)
    } else {
        bail!("plugin '{name}' not found in {owner}/{repo} (tried root and {name}/ subdir)");
    };

    let toml_path = if prefix.is_empty() {
        format!("{name}.toml")
    } else {
        format!("{prefix}/{name}.toml")
    };
    let toml_opt = fetch_optional(&raw_url(owner, repo, &toml_path));

    install_one_in(dir, name, ext, &script, toml_opt, force)?;
    println!("installed {name}");
    Ok(())
}

fn install_all(owner: &str, repo: &str, dir: &Path, force: bool) -> anyhow::Result<()> {
    let files = list_repo_plugins(owner, repo)?;
    if files.is_empty() {
        bail!("no .sh/.py plugins found in {owner}/{repo}");
    }

    let mut installed = 0usize;
    let mut errors = 0usize;

    for file in &files {
        let stem = Path::new(&file.name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&file.name)
            .to_string();
        let ext = Path::new(&file.name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("sh")
            .to_string();

        let script = match curl_fetch(&raw_url(owner, repo, &file.path)) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("skip {stem}: {e}");
                errors += 1;
                continue;
            }
        };

        // Try to find a sidecar at the same directory level.
        let toml_path = {
            let parent = Path::new(&file.path)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("");
            if parent.is_empty() {
                format!("{stem}.toml")
            } else {
                format!("{parent}/{stem}.toml")
            }
        };
        let toml_opt = fetch_optional(&raw_url(owner, repo, &toml_path));

        match install_one_in(dir, &stem, &ext, &script, toml_opt, force) {
            Ok(()) => {
                println!("installed {stem}");
                installed += 1;
            }
            Err(e) => {
                eprintln!("skip {stem}: {e}");
                errors += 1;
            }
        }
    }

    if installed == 0 {
        bail!("no plugins were installed ({errors} errors)");
    }
    if errors > 0 {
        println!("{installed} plugin(s) installed, {errors} skipped");
    } else {
        println!("{installed} plugin(s) installed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::{install_one_in, parse_source};

    #[test]
    fn parse_source_two_segments() {
        let (owner, repo, name) = parse_source("alice/widgets").unwrap();
        assert_eq!(owner, "alice");
        assert_eq!(repo, "widgets");
        assert!(name.is_none());
    }

    #[test]
    fn parse_source_three_segments() {
        let (owner, repo, name) = parse_source("alice/widgets/myplugin").unwrap();
        assert_eq!(owner, "alice");
        assert_eq!(repo, "widgets");
        assert_eq!(name.as_deref(), Some("myplugin"));
    }

    #[test]
    fn parse_source_invalid() {
        assert!(parse_source("onlyone").is_err());
        assert!(parse_source("").is_err());
    }

    #[test]
    fn install_one_rejects_path_traversal() {
        let dir = TempDir::new().unwrap();
        assert!(
            install_one_in(dir.path(), "../../.bashrc", "sh", b"#!/bin/sh", None, false)
                .unwrap_err()
                .to_string()
                .contains("unsafe plugin name")
        );
        assert!(
            install_one_in(dir.path(), ".hidden", "sh", b"#!/bin/sh", None, false)
                .unwrap_err()
                .to_string()
                .contains("unsafe plugin name")
        );
    }

    #[test]
    fn install_one_writes_script_and_sidecar() {
        let dir = TempDir::new().unwrap();
        install_one_in(
            dir.path(),
            "myplugin",
            "sh",
            b"#!/bin/sh\necho hi",
            None,
            false,
        )
        .unwrap();

        assert!(dir.path().join("myplugin.sh").exists());
        assert!(dir.path().join("myplugin.toml").exists());

        let content = std::fs::read_to_string(dir.path().join("myplugin.sh")).unwrap();
        assert_eq!(content, "#!/bin/sh\necho hi");
    }

    #[test]
    fn install_one_rejects_builtin_collision() {
        let dir = TempDir::new().unwrap();
        // "git" is in widgets::AVAILABLE
        let err = install_one_in(dir.path(), "git", "sh", b"#!/bin/sh", None, false).unwrap_err();
        assert!(err.to_string().contains("built-in widget"));
    }

    #[test]
    fn install_one_rejects_duplicate_without_force() {
        let dir = TempDir::new().unwrap();
        install_one_in(
            dir.path(),
            "myplugin",
            "sh",
            b"#!/bin/sh\necho v1",
            None,
            false,
        )
        .unwrap();
        let err = install_one_in(
            dir.path(),
            "myplugin",
            "sh",
            b"#!/bin/sh\necho v2",
            None,
            false,
        )
        .unwrap_err();
        assert!(err.to_string().contains("already installed"));
    }

    #[test]
    fn install_one_overwrites_with_force() {
        let dir = TempDir::new().unwrap();
        install_one_in(
            dir.path(),
            "myplugin",
            "sh",
            b"#!/bin/sh\necho v1",
            None,
            false,
        )
        .unwrap();
        install_one_in(
            dir.path(),
            "myplugin",
            "sh",
            b"#!/bin/sh\necho v2",
            None,
            true,
        )
        .unwrap();

        let content = std::fs::read_to_string(dir.path().join("myplugin.sh")).unwrap();
        assert_eq!(content, "#!/bin/sh\necho v2");
    }
}
