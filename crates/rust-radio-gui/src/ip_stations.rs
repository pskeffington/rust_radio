//! Bounded, host-owned IP station storage. Import never connects or starts audio.
use super::RadioPanel;
use rust_radio_core::directory::{curated_stations, station_by_id};
use serde::{Deserialize, Serialize};

pub(super) const MAX_STATIONS: usize = 24;
pub(super) const MAX_BYTES: usize = 64 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SavedStation {
    pub name: String,
    pub url: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u8,
    stations: Vec<SavedStation>,
    selected: Selection,
}
#[derive(Serialize, Deserialize)]
enum Selection {
    Curated(String),
    Saved(usize),
}
fn validate(station: &SavedStation) -> Result<(), String> {
    if station.name.trim().is_empty()
        || station.name != station.name.trim()
        || station.name.len() > 96
        || station.name.chars().any(char::is_control)
    {
        return Err("Station name must contain 1–96 bytes without control characters".into());
    }
    rust_radio_ip::net::validate_direct_url(&station.url)
}
impl RadioPanel {
    /// Replace saved IP stations atomically. No network, file IO, or autoplay.
    /// Reject imports while playback or a queued station switch is active.
    pub fn load_ip_stations(&mut self, json: &str) -> Result<(), String> {
        if self.engine.is_some() || self.restart_pending || self.validation.is_some() {
            return Err("Stop playback before loading stations".into());
        }
        if json.len() > MAX_BYTES {
            return Err("IP stations exceed 64 KiB".into());
        }
        let document: Document =
            serde_json::from_str(json).map_err(|_| "Invalid IP station file")?;
        if document.version != 1 || document.stations.len() > MAX_STATIONS {
            return Err("Unsupported IP station version or too many stations".into());
        }
        for (i, station) in document.stations.iter().enumerate() {
            validate(station)?;
            if document.stations[..i]
                .iter()
                .any(|other| other.name.trim().to_lowercase() == station.name.trim().to_lowercase())
            {
                return Err("Duplicate station name".into());
            }
        }
        match &document.selected {
            Selection::Curated(id) if station_by_id(id).is_none() => {
                return Err("Unknown selected station".into())
            }
            Selection::Saved(index) if *index >= document.stations.len() => {
                return Err("Invalid selected station".into())
            }
            _ => {}
        }
        self.ip_stations = document.stations;
        match document.selected {
            Selection::Curated(id) => {
                self.saved_station_index = None;
                self.select_source(
                    curated_stations()
                        .iter()
                        .position(|station| station.id == id)
                        .unwrap(),
                );
            }
            Selection::Saved(index) => self.select_ip_station(curated_stations().len() + index),
        }
        self.ip_stations_dirty = false;
        self.selection_check_pending = None;
        Ok(())
    }
    /// Hosts persist this JSON when Some; URLs are saved only by explicit Save.
    pub fn take_ip_stations_update(&mut self) -> Option<String> {
        if !std::mem::take(&mut self.ip_stations_dirty) {
            return None;
        }
        let selected = self
            .saved_station_index
            .map(Selection::Saved)
            .unwrap_or_else(|| Selection::Curated(self.selected_station().id.into()));
        serde_json::to_string_pretty(&Document {
            version: 1,
            stations: self.ip_stations.clone(),
            selected,
        })
        .ok()
    }
    pub fn report_ip_stations_saved(&mut self, result: Result<(), String>) {
        if let Err(error) = result {
            self.note = error;
        }
    }
    pub(super) fn save_ip_station(&mut self) -> Result<(), String> {
        let station = SavedStation {
            name: self.ip_station_name.trim().into(),
            url: self.direct_url.trim().into(),
        };
        validate(&station)?;
        let existing = self
            .ip_stations
            .iter()
            .position(|other| other.name.to_lowercase() == station.name.to_lowercase());
        let index = if let Some(index) = existing {
            self.ip_stations[index] = station;
            index
        } else {
            if self.ip_stations.len() >= MAX_STATIONS {
                return Err("Station bank is full (24 maximum)".into());
            }
            self.ip_stations.push(station);
            self.ip_stations.len() - 1
        };
        self.saved_station_index = Some(index);
        self.ip_stations_dirty = true;
        Ok(())
    }
    pub(super) fn ip_station_count(&self) -> usize {
        curated_stations().len() + self.ip_stations.len() + 1
    }
    pub(super) fn ip_station_index(&self) -> usize {
        if self.direct_selected {
            self.saved_station_index
                .map(|i| curated_stations().len() + i)
                .unwrap_or(self.ip_station_count() - 1)
        } else {
            self.station_index
        }
    }
    pub(super) fn select_ip_station(&mut self, index: usize) {
        if index < curated_stations().len() {
            self.saved_station_index = None;
            self.select_source(index);
        } else {
            let saved = index - curated_stations().len();
            let next = self.ip_stations.get(saved).cloned();
            let changed = self.ip_station_index() != index;
            self.saved_station_index = next.as_ref().map(|_| saved);
            if let Some(station) = next {
                self.direct_url = station.url;
                self.ip_station_name = station.name;
            }
            self.select_source(
                curated_stations().len()
                    + usize::from(cfg!(feature = "sdrconnect"))
                    + usize::from(cfg!(feature = "soapy-sdr")),
            );
            // All saved entries share the Direct source index; URL changes still
            // supersede the old worker, including a switch already cleaning up.
            if changed {
                self.restart_ip_if_active();
            }
            if self.saved_station_index.is_none() {
                self.settings_open = true;
            }
        }
        self.ip_stations_dirty = true;
    }
    pub(super) fn ip_station_name(&self) -> &str {
        if self.direct_selected {
            self.saved_station_index
                .and_then(|i| self.ip_stations.get(i))
                .map(|s| s.name.as_str())
                .unwrap_or("Custom public / LAN stream")
        } else {
            self.selected_station().name
        }
    }
}
