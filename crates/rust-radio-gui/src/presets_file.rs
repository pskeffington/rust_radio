//! Standalone host persistence. The embeddable panel only exports bounded JSON.
use rust_radio_gui::RadioPanel;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;
const MAX_BYTES: u64 = 64 * 1024;

#[derive(Clone, Copy)]
enum Kind {
    Ip,
    #[cfg(feature = "nrsp-native")]
    Fm,
}

pub(super) struct PresetFile {
    path: Option<PathBuf>,
    kind: Kind,
    pending: Option<String>,
    worker: Option<JoinHandle<Result<(), String>>>,
}
impl PresetFile {
    #[cfg(feature = "nrsp-native")]
    pub fn load(panel: &mut RadioPanel) -> Self {
        Self::load_kind(panel, Kind::Fm)
    }
    pub fn load_ip(panel: &mut RadioPanel) -> Self {
        Self::load_kind(panel, Kind::Ip)
    }
    fn load_kind(panel: &mut RadioPanel, kind: Kind) -> Self {
        let path = path(kind);
        if let Some(path) = &path {
            match read(path) {
                Ok(Some(json)) => {
                    let result = match kind {
                        Kind::Ip => panel.load_ip_stations(&json),
                        #[cfg(feature = "nrsp-native")]
                        Kind::Fm => panel.load_fm_presets(&json),
                    };
                    report(panel, kind, result);
                }
                Ok(None) => {}
                Err(error) => report(panel, kind, Err(error)),
            }
        }
        Self {
            path,
            kind,
            pending: None,
            worker: None,
        }
    }
    pub fn poll(&mut self, panel: &mut RadioPanel) {
        let update = match self.kind {
            Kind::Ip => panel.take_ip_stations_update(),
            #[cfg(feature = "nrsp-native")]
            Kind::Fm => panel.take_fm_presets_update(),
        };
        if let Some(json) = update {
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
            report(panel, self.kind, result);
        }
        if self.worker.is_none() {
            if let Some(json) = self.pending.take() {
                if let Some(path) = self.path.clone() {
                    match std::thread::Builder::new()
                        .name("rust-radio-presets".into())
                        .spawn(move || write(&path, &json))
                    {
                        Ok(worker) => self.worker = Some(worker),
                        Err(error) => report(panel, self.kind, Err(error.to_string())),
                    }
                } else {
                    report(
                        panel,
                        self.kind,
                        Err("No local configuration directory; stations remain in memory".into()),
                    );
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
fn report(panel: &mut RadioPanel, kind: Kind, result: Result<(), String>) {
    match kind {
        Kind::Ip => panel.report_ip_stations_saved(result),
        #[cfg(feature = "nrsp-native")]
        Kind::Fm => panel.report_fm_presets_saved(result),
    }
}
fn path(kind: Kind) -> Option<PathBuf> {
    let (variable, file) = match kind {
        Kind::Ip => ("RUST_RADIO_STATIONS_PATH", "ip-stations.json"),
        #[cfg(feature = "nrsp-native")]
        Kind::Fm => ("RUST_RADIO_PRESETS_PATH", "fm-presets.json"),
    };
    if let Some(path) = std::env::var_os(variable) {
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
    Some(base.join("rust_radio").join(file))
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
        .map_err(|e| e.to_string())?;
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
    result.map_err(|e: std::io::Error| format!("Cannot save stations: {e}"))
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
