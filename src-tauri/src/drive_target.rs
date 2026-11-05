use keroro_encoder_core::{
    session,
    storage::{self, DriveInfo},
};
use std::path::PathBuf;

pub async fn connected() -> Result<Vec<DriveInfo>, String> {
    session::blocking(storage::connected).await
}
pub async fn inspect(path: String) -> Result<DriveInfo, String> {
    session::blocking(move || storage::inspect(&PathBuf::from(path))).await
}
pub async fn prepare(id: String) -> Result<String, String> {
    session::blocking(move || storage::prepare(&id).map(|path| path.to_string_lossy().into_owned()))
        .await
}
