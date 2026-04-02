use anyhow::bail;

pub(crate) fn curl_fetch(url: &str) -> anyhow::Result<Vec<u8>> {
    let out = std::process::Command::new("curl")
        .args([
            "--fail",
            "-s",
            "--max-time",
            "10",
            "-H",
            "User-Agent: soffit",
            url,
        ])
        .output()?;
    if !out.status.success() {
        bail!(
            "curl failed for {url}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(out.stdout)
}
