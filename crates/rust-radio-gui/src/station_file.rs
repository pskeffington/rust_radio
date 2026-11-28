//! Bounded standalone persistence for operator-saved IP stations.
use rust_radio_gui::RadioPanel;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;

const MAX_BYTES: u64 = 64 * 1024;

pub(super) struct StationFile {
    path: Option<PathBuf>,
    pending: Option<String>,
    worker: Option<JoinHandle<Result<(), String>>>,
}

impl StationFile {
    pub fn load(panel: &mut RadioPanel) -> Self {
        let path = path();
        if let Some(path) = &path {
            match read(path) {
                Ok(Some(json)) => {
                    if let Err(error) = panel.load_ip_stations(&json) {
                        panel.report_ip_stations_saved(Err(error));
                    }
                }
                Ok(None) => {}
                Err(error) => panel.report_ip_stations_saved(Err(error)),
            }
        }
        Self {
            path,
            pending: None,
            worker: None,
        }
    }

    pub fn poll(&mut self, panel: &mut RadioPanel) {
        if let Some(json) = panel.take_ip_stations_update() {
            self.pending = Some(json);
        }
        if self.worker.as_ref().is_some_and(JoinHandle::is_finished) {
            let result = self
                .worker
                .take()
                .expect("finished station writer")
                .join()
                .unwrap_or_else(|_| Err("Station writer failed".into()));
            panel.report_ip_stations_saved(result);
        }
        if self.worker.is_none() {
            if let Some(json) = self.pending.take() {
                if let Some(path) = self.path.clone() {
                    match std::thread::Builder::new()
                        .name("rust-radio-stations".into())
                        .spawn(move || write(&path, &json))
                    {
                        Ok(worker) => self.worker = Some(worker),
                        Err(error) => panel.report_ip_stations_saved(Err(error.to_string())),
                    }
                } else {
                    panel.report_ip_stations_saved(Err(
                        "No local configuration directory; stations remain in memory".into(),
                    ));
                }
            }
        }
    }
}

impl Drop for StationFile {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        if let (Some(path), Some(json)) = (&self.path, self.pending.take()) {
            let _ = write(path, &json);
        }
    }
}

fn path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("RUST_RADIO_STATIONS_PATH") {
        return Some(path.into());
    }
    #[cfg(target_os = "macos")]
    let base = PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support");
    #[cfg(target_os = "windows")]
    let base = PathBuf::from(std::env::var_os("APPDATA")?);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("rust_radio").join("ip-stations.json"))
}

fn read(path: &Path) -> Result<Option<String>, String> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot load stations: {error}")),
    };
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Station file exceeds 64 KiB".into());
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| "Station file is not UTF-8".into())
}

fn write(path: &Path, json: &str) -> Result<(), String> {
    if json.len() as u64 > MAX_BYTES {
        return Err("Station file exceeds 64 KiB".into());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let temp = path.with_extension(format!("tmp-{}-{nonce}", std::process::id()));
    let result = (|| {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.map_err(|error: std::io::Error| format!("Cannot save stations: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rust-radio-station-file-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn bounded_atomic_station_file_roundtrips_and_rejects_oversize() {
        let dir = temp_dir();
        let path = dir.join("ip-stations.json");
        let document = r#"{"version":1,"stations":[],"selected":{"Curated":"skylab"}}"#;
        write(&path, document).unwrap();
        assert_eq!(read(&path).unwrap().as_deref(), Some(document));
        assert!(write(&path, &"x".repeat(MAX_BYTES as usize + 1)).is_err());
        assert_eq!(read(&path).unwrap().as_deref(), Some(document));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_station_document_is_rejected_by_panel_import() {
        let mut panel = RadioPanel::new();
        assert!(panel.load_ip_stations("not json").is_err());
        assert!(panel
            .load_ip_stations(&"x".repeat(MAX_BYTES as usize + 1))
            .is_err());
        assert!(!panel.is_active());
    }
}
