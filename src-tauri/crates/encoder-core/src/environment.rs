use std::path::Path;

/// Presence/access check only. Metadata does not verify a cryptographic signature.
pub async fn validate_binary(path: &Path, name: &str) -> Result<(), String> {
    let metadata = tokio::fs::metadata(path).await.map_err(|error|
        format!("ENVIRONMENT_UNAVAILABLE: Bundled {name} is missing or inaccessible: {} ({error}). Restore the complete application package.", path.display()))?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(format!(
            "ENVIRONMENT_UNAVAILABLE: Bundled {name} is not a nonempty regular file: {}. Restore the complete application package.",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn missing_empty_and_directory_binaries_are_rejected() {
        let root = tempfile::tempdir().expect("fixture");
        let binary = root.path().join("ffmpeg.exe");
        assert!(
            validate_binary(&binary, "ffmpeg")
                .await
                .err()
                .expect("missing")
                .contains("ENVIRONMENT_UNAVAILABLE")
        );
        tokio::fs::write(&binary, []).await.expect("empty");
        assert!(validate_binary(&binary, "ffmpeg").await.is_err());
        assert!(validate_binary(root.path(), "ffmpeg").await.is_err());
        tokio::fs::write(&binary, b"MZfixture")
            .await
            .expect("nonempty");
        assert!(validate_binary(&binary, "ffmpeg").await.is_ok());
    }
}
