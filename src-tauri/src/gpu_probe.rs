use crate::execution::resolve_tools;
use keroro_encoder_core::{ActiveProcess, Cancellation, hardware};
use std::sync::Arc;
use tauri::AppHandle;

pub async fn detect_gpu_encoders(app: &AppHandle) -> Result<Vec<String>, String> {
    static CACHE: tokio::sync::Mutex<Option<Vec<String>>> = tokio::sync::Mutex::const_new(None);
    let mut cache = CACHE.lock().await;
    if let Some(value) = cache.as_ref() {
        return Ok(value.clone());
    }
    let tools = resolve_tools(app).await?;
    let processes = Arc::new(ActiveProcess::default());
    let encoders = hardware::available(&tools, &Cancellation::new(), &processes).await?;
    let result: Vec<_> = encoders
        .into_iter()
        .map(|encoder| encoder.name().to_owned())
        .collect();
    *cache = Some(result.clone());
    Ok(result)
}
