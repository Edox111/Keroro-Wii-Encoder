//! Filter argument escaping, independent of shell quoting (no shell is used).
use std::path::Path;
fn escape(value: &str, special: &str) -> String {
    let mut result = String::new();
    for c in value.chars() {
        if special.contains(c) {
            result.push('\\');
        }
        result.push(c);
    }
    result
}
pub fn embedded_subtitle_filter(path: &Path, index: u32) -> Result<String, String> {
    if index >= 256 {
        return Err("Invalid embedded subtitle index.".into());
    }
    let path = path
        .to_str()
        .ok_or("Subtitle source path must be valid Unicode.")?;
    if path.chars().any(|c| c.is_control()) {
        return Err("Subtitle path contains control characters.".into());
    }
    #[cfg(windows)]
    let path = path
        .strip_prefix(r"\\?\")
        .unwrap_or(path)
        .replace('\\', "/");
    #[cfg(not(windows))]
    let path = path.to_owned();
    // First libavfilter option parsing, then filtergraph parsing. No shell layer.
    let option = escape(&path, "\\':");
    let graph = escape(&option, "\\'[],; ");
    Ok(format!("subtitles=filename={graph}:si={index}"))
}
