use log::{LevelFilter, Log, Metadata, Record};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

const LIMIT: u64 = 16 * 1024 * 1024;
struct Sink {
    file: Option<File>,
    bytes: u64,
}
struct LocalLogger {
    path: PathBuf,
    sink: Mutex<Sink>,
}
static LOGGER: OnceLock<LocalLogger> = OnceLock::new();

impl LocalLogger {
    fn write(&self, line: &[u8]) {
        let Ok(mut sink) = self.sink.try_lock() else {
            // A panic while another record is being written must not deadlock.
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
            {
                let _ = file.write_all(line);
                let _ = file.sync_data();
            }
            return;
        };
        let mut rotation_error = None;
        if sink.bytes.saturating_add(line.len() as u64) > LIMIT {
            sink.file.take();
            let backup = self.path.with_extension("log.1");
            rotation_error = fs::rename(&self.path, &backup).err();
            sink.file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .ok();
            sink.bytes = fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
        }
        if let Some(file) = sink.file.as_mut() {
            // Logging through log! here would recurse into this locked sink.
            if let Some(error) = rotation_error {
                let _ = writeln!(file, "[WARN] [logger] Cannot rotate local log: {error}");
            }
            if file.write_all(line).and_then(|_| file.flush()).is_ok() {
                sink.bytes = sink.bytes.saturating_add(line.len() as u64);
            }
        }
    }
}
impl Log for LocalLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Info
    }
    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|t| t.as_millis())
            .unwrap_or(0);
        self.write(
            format!(
                "{timestamp} [{}] [{}] {}\n",
                record.level(),
                record.target(),
                record.args()
            )
            .as_bytes(),
        );
    }
    fn flush(&self) {
        if let Ok(mut sink) = self.sink.try_lock()
            && let Some(file) = sink.file.as_mut()
        {
            let _ = file.sync_data();
        }
    }
}

pub fn install(directory: &Path) -> Result<PathBuf, String> {
    crate::offline::local_path(directory)?;
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    crate::artifacts::no_links(directory)?;
    let path = directory.join("keroro-encoder.log");
    if path.exists() {
        crate::artifacts::no_links(&path)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("Cannot open local log: {e}"))?;
    let bytes = file.metadata().map_err(|e| e.to_string())?.len();
    LOGGER
        .set(LocalLogger {
            path: path.clone(),
            sink: Mutex::new(Sink {
                file: Some(file),
                bytes,
            }),
        })
        .map_err(|_| "Local logger already initialized.")?;
    let logger = LOGGER.get().ok_or("Local logger unavailable.")?;
    log::set_logger(logger).map_err(|e| e.to_string())?;
    log::set_max_level(LevelFilter::Info);
    std::panic::set_hook(Box::new(|panic| {
        let trace = std::backtrace::Backtrace::force_capture();
        log::error!(target: "panic", "{panic}\n{trace}");
        log::logger().flush();
    }));
    log::info!("Local logging initialized; no telemetry or network log sink.");
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn file_sink_writes_only_to_its_local_file() {
        let root = tempfile::tempdir().expect("fixture");
        let path = root.path().join("keroro-encoder.log");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .expect("open");
        let logger = LocalLogger {
            path: path.clone(),
            sink: Mutex::new(Sink {
                file: Some(file),
                bytes: 0,
            }),
        };
        logger.write(b"local diagnostic\n");
        logger.flush();
        assert_eq!(
            fs::read_to_string(path).expect("read"),
            "local diagnostic\n"
        );
    }
}
