use std::process::Command;

pub fn head() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let hash = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if hash.is_empty() { None } else { Some(hash) }
}

pub fn exists(hash: &str) -> Option<bool> {
    head()?;
    let out = Command::new("git")
        .args(["cat-file", "-e", hash])
        .output()
        .ok()?;
    Some(out.status.success())
}
