//! Embeddable 480x320 radio surface. The host owns the window and event loop.
//! Constructing a panel never opens an audio device or connects to a broadcaster.
use eframe::egui;
use rust_radio_audio::compat_controller::PlaybackState;
use rust_radio_audio::output::{available_outputs, OutputDeviceInfo};
use rust_radio_audio::playback::NativeRadioEngine;
use rust_radio_core::directory::{curated_stations, station_by_id};

pub const PANEL_WIDTH: f32 = 480.0;
pub const PANEL_HEIGHT: f32 = 320.0;
const GREEN: egui::Color32 = egui::Color32::from_rgb(151, 240, 139);
const DARK: egui::Color32 = egui::Color32::from_rgb(9, 22, 14);

/// Owns one radio session. Embed with `panel.show(ui)` inside an existing HUD.
pub struct RadioPanel {
    station: String,
    volume: u8,
    output: String,
    outputs: Vec<OutputDeviceInfo>,
    engine: Option<NativeRadioEngine>,
    note: String,
    show_output: bool,
    stopping: bool,
}

impl Default for RadioPanel {
    fn default() -> Self {
        Self {
            station: curated_stations()
                .first()
                .map(|s| s.id.to_owned())
                .unwrap_or_default(),
            volume: 60,
            output: "system".into(),
            outputs: Vec::new(),
            engine: None,
            note: "READY / SELECT STATION AND PRESS PLAY".into(),
            show_output: false,
            stopping: false,
        }
    }
}

impl RadioPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin cancellation without blocking the host UI. Keep rendering the panel
    /// (or call `poll_shutdown`) until `is_active` becomes false before dismounting.
    pub fn stop(&mut self) {
        if let Some(engine) = &self.engine {
            engine.request_stop();
            self.stopping = true;
            self.note = "STOPPING / RELEASING RECEIVER".into();
        }
        self.poll_shutdown();
    }

    /// Let hosts complete shutdown even while this panel is hidden.
    /// Returns true when no receiver resources remain owned by the panel.
    pub fn poll_shutdown(&mut self) -> bool {
        if !self.stopping {
            return self.engine.is_none();
        }
        let finished = self.engine.as_mut().is_none_or(|engine| engine.try_finish());
        if finished {
            self.engine = None;
            self.stopping = false;
            self.note = "RECEIVER STOPPED".into();
        }
        self.engine.is_none()
    }

    pub fn is_active(&self) -> bool {
        self.engine.is_some()
    }

    fn play(&mut self) {
        if self.engine.is_some() {
            return;
        }
        if station_by_id(&self.station).is_none() {
            self.note = "UNKNOWN STATION".into();
            return;
        }
        match NativeRadioEngine::start_internet(&self.station, self.volume, &self.output) {
            Ok(engine) => {
                self.engine = Some(engine);
                self.note = "CONNECTING / NATIVE AUDIO".into();
            }
            Err(error) => self.note = error,
        }
    }

    fn refresh_outputs(&mut self) {
        match available_outputs() {
            Ok(outputs) => {
                self.outputs = outputs;
                self.note = "SELECT OUTPUT / STOP BEFORE SWITCHING".into();
            }
            Err(error) => self.note = error,
        }
    }

    /// Pure touch-oriented presentation except when the user explicitly clicks Play,
    /// Stop, output refresh, or adjusts the volume. No network traffic on render.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.poll_shutdown();
        let frame = egui::Frame::new()
            .fill(DARK)
            .stroke(egui::Stroke::new(1.0_f32, GREEN))
            .inner_margin(egui::Margin::same(9));
        frame.show(ui, |ui| {
            ui.set_max_width((ui.available_width() - 2.0).min(PANEL_WIDTH - 20.0));
            ui.visuals_mut().override_text_color = Some(GREEN);
            ui.spacing_mut().item_spacing = egui::vec2(7.0, 7.0);
            ui.spacing_mut().button_padding = egui::vec2(10.0, 8.0);

            let status = self.engine.as_ref().map(|engine| engine.status());
            let state = status
                .as_ref()
                .map(|s| s.state)
                .unwrap_or(PlaybackState::Stopped);
            let running = self.engine.is_some()
                && !matches!(state, PlaybackState::Failed | PlaybackState::Stopped);

            ui.horizontal(|ui| {
                ui.strong("RADIO  /  IP");
                ui.add_space(12.0);
                ui.label(if self.stopping {
                    "STOPPING".into()
                } else {
                    format!("{:?}", state).to_uppercase()
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("RUST RADIO");
                });
            });
            ui.separator();

            ui.add_enabled_ui(!running && !self.stopping, |ui| {
                ui.horizontal(|ui| {
                    if ui.add_sized([45.0, 43.0], egui::Button::new("◀")).clicked() {
                        self.step_station(-1);
                    }
                    egui::ComboBox::from_id_salt("rust_radio_station")
                        .width(260.0)
                        .selected_text(
                            station_by_id(&self.station)
                                .map(|s| s.name)
                                .unwrap_or("Select station"),
                        )
                        .show_ui(ui, |ui| {
                            for station in curated_stations() {
                                ui.selectable_value(
                                    &mut self.station,
                                    station.id.to_owned(),
                                    station.name,
                                );
                            }
                        });
                    if ui.add_sized([45.0, 43.0], egui::Button::new("▶")).clicked() {
                        self.step_station(1);
                    }
                });
            });
            if let Some(station) = station_by_id(&self.station) {
                ui.label(format!("{}  /  {}", station.category, station.genre));
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !running && !self.stopping,
                        egui::Button::new("▶  PLAY").min_size(egui::vec2(116.0, 44.0)),
                    )
                    .clicked()
                {
                    if self.engine.is_some() {
                        self.stop();
                    }
                    self.play();
                }
                if ui
                    .add_enabled(
                        self.engine.is_some() && !self.stopping,
                        egui::Button::new("■  STOP").min_size(egui::vec2(116.0, 44.0)),
                    )
                    .clicked()
                {
                    self.stop();
                }
                ui.add_space(4.0);
                ui.label(format!("VOL {}%", self.volume));
            });
            let slider = ui.add_sized(
                [ui.available_width(), 26.0],
                egui::Slider::new(&mut self.volume, 0..=100).show_value(false),
            );
            if slider.changed() {
                if let Some(engine) = &self.engine {
                    if let Err(error) = engine.set_volume(self.volume) {
                        self.note = error;
                    }
                }
            }
            if let Some(status) = status {
                if let Some(error) = status.last_error {
                    self.note = error;
                }
            }
            if matches!(state, PlaybackState::Failed) && self.note == "CONNECTING / NATIVE AUDIO" {
                self.note = "STREAM FAILED / STOP THEN RETRY".into();
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add_sized([120.0, 38.0], egui::Button::new("OUTPUT ▾"))
                    .clicked()
                {
                    self.show_output = !self.show_output;
                    if self.show_output {
                        self.refresh_outputs();
                    }
                }
                ui.label(egui::RichText::new(&self.note).small());
            });
            if self.show_output {
                egui::Window::new("RADIO / AUDIO OUTPUT")
                    .id(egui::Id::new("rust_radio_output_overlay"))
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .fixed_size(egui::vec2(380.0, 165.0))
                    .resizable(false)
                    .collapsible(false)
                    .show(ui.ctx(), |ui| {
                        ui.label("Stop playback before selecting another output.");
                        ui.add_enabled_ui(self.engine.is_none(), |ui| {
                            egui::ComboBox::from_id_salt("rust_radio_output")
                                .width(325.0)
                                .selected_text(
                                    self.outputs
                                        .iter()
                                        .find(|d| d.id == self.output)
                                        .map(|d| d.name.as_str())
                                        .unwrap_or("System default"),
                                )
                                .show_ui(ui, |ui| {
                                    for device in &self.outputs {
                                        ui.add_enabled_ui(device.available, |ui| {
                                            ui.selectable_value(
                                                &mut self.output,
                                                device.id.clone(),
                                                &device.name,
                                            );
                                        });
                                    }
                                });
                        });
                        ui.horizontal(|ui| {
                            if ui.button("REFRESH").clicked() {
                                self.refresh_outputs();
                            }
                            if ui.button("SYSTEM DEFAULT").clicked() && self.engine.is_none() {
                                self.output = "system".into();
                            }
                            if ui.button("CLOSE").clicked() {
                                self.show_output = false;
                            }
                        });
                    });
            }
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        });
    }

    fn step_station(&mut self, direction: isize) {
        let list = curated_stations();
        if list.is_empty() {
            return;
        }
        let index = list.iter().position(|s| s.id == self.station).unwrap_or(0) as isize;
        let next = (index + direction).rem_euclid(list.len() as isize) as usize;
        self.station = list[next].id.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopping_an_idle_panel_is_immediate_and_idempotent() {
        let mut panel = RadioPanel::new();
        panel.stop();
        panel.stop();
        assert!(panel.poll_shutdown());
        assert!(!panel.is_active());
        assert!(!panel.stopping);
    }

    #[test]
    fn panel_constructs_without_starting_radio() {
        let panel = RadioPanel::new();
        assert!(!panel.is_active());
        assert!(station_by_id(&panel.station).is_some());
    }
    #[test]
    fn changing_station_is_offline() {
        let mut panel = RadioPanel::new();
        let original = panel.station.clone();
        panel.step_station(1);
        assert_ne!(original, panel.station);
        assert!(!panel.is_active());
    }
}
