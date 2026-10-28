//! Standalone host persistence. The embeddable panel only exports bounded JSON.
use rust_radio_gui::RadioPanel;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;
const MAX_BYTES: u64 = 16 * 1024;

pub(super) struct PresetFile {
    path: Option<PathBuf>,
    pending: Option<String>,
    worker: Option<JoinHandle<Result<(), String>>>,
}
impl PresetFile {
    pub fn load(panel: &mut RadioPanel) -> Self {
        let path = path();
        if let Some(path) = &path {
            match read(path) {
                Ok(Some(json)) => {
                    if let Err(error) = panel.load_fm_presets(&json) {
                        panel.report_fm_presets_saved(Err(error));
                    }
                }
                Ok(None) => {}
                Err(error) => panel.report_fm_presets_saved(Err(error)),
            }
        }
        Self {
            path,
            pending: None,
            worker: None,
        }
    }
    pub fn poll(&mut self, panel: &mut RadioPanel) {
        if let Some(json) = panel.take_fm_presets_update() {
            self.pending = Some(json);
        }
        if self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
        {
            let result = self
                .worker
                .take()
                .expect("finished writer")
                .join()
                .unwrap_or_else(|_| Err("Preset writer failed".into()));
            panel.report_fm_presets_saved(result);
        }
        if self.worker.is_none() {
            if let Some(json) = self.pending.take() {
                if let Some(path) = self.path.clone() {
                    match std::thread::Builder::new()
                        .name("rust-radio-presets".into())
                        .spawn(move || write(&path, &json))
                    {
                        Ok(worker) => self.worker = Some(worker),
                        Err(error) => panel.report_fm_presets_saved(Err(error.to_string())),
                    }
                } else {
                    panel.report_fm_presets_saved(Err(
                        "No local configuration directory; presets remain in memory".into(),
                    ));
                }
            }
        }
    }
}
impl Drop for PresetFile {
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
    if let Some(path) = std::env::var_os("RUST_RADIO_PRESETS_PATH") {
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
    Some(base.join("rust_radio/fm-presets.json"))
}
fn read(path: &Path) -> Result<Option<String>, String> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot load FM presets: {error}")),
    };
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("FM preset file exceeds 16 KiB".into());
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| "FM preset file is not UTF-8".into())
}
fn write(path: &Path, json: &str) -> Result<(), String> {
    if json.len() as u64 > MAX_BYTES {
        return Err("FM presets exceed 16 KiB".into());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
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
    result.map_err(|e: std::io::Error| format!("Cannot save FM presets: {e}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preset_file_replaces_atomically_and_rejects_oversize_without_damage() {
        let path = std::env::temp_dir().join(format!(
            "rust-radio-presets-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write(&path, "first").unwrap();
        write(&path, "second").unwrap();
        assert_eq!(read(&path).unwrap(), Some("second".into()));
        assert!(write(&path, &"x".repeat(MAX_BYTES as usize + 1)).is_err());
        assert_eq!(read(&path).unwrap(), Some("second".into()));
        std::fs::remove_file(path).unwrap();
    }
}
