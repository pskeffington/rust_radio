use super::{RadioPanel, PANEL_WIDTH};
use eframe::egui;
use rust_radio_audio::compat_controller::PlaybackState;
use rust_radio_core::directory::curated_stations;

const GREEN: egui::Color32 = egui::Color32::from_rgb(151, 240, 139);
const DARK: egui::Color32 = egui::Color32::from_rgb(9, 22, 14);

impl RadioPanel {
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

            let status = self
                .engine
                .as_ref()
                .map(|engine| engine.status())
                .or_else(|| self.last_session.clone());
            let state = status
                .as_ref()
                .map(|s| s.state)
                .unwrap_or(PlaybackState::Stopped);
            let running = self.engine.is_some();
            let busy = self.is_active();

            if let Some(error) = status
                .as_ref()
                .and_then(|session| session.last_error.as_ref())
            {
                self.note = error.clone();
            }
            if matches!(state, PlaybackState::Failed) && self.note == "CONNECTING / NATIVE AUDIO" {
                self.note = "STREAM FAILED / PRESS PLAY TO RETRY".into();
            }

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
                        .truncate()
                        .selected_text(self.selected_station().name)
                        .show_ui(ui, |ui| {
                            for (index, station) in curated_stations().iter().enumerate() {
                                let label = format!(
                                    "{} [{}]",
                                    station.name,
                                    self.validation_label(station.id)
                                );
                                ui.selectable_value(&mut self.station_index, index, label);
                            }
                        });
                    if ui.add_sized([45.0, 43.0], egui::Button::new("▶")).clicked() {
                        self.step_station(1);
                    }
                });
            });
            self.cancel_stale_validation();
            {
                let station = self.selected_station();
                ui.add(
                    egui::Label::new(format!(
                        "{} / {} / {}",
                        self.validation_label(station.id),
                        station.category,
                        station.genre
                    ))
                    .truncate(),
                );
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !busy && !self.stopping,
                        egui::Button::new("▶  PLAY").min_size(egui::vec2(116.0, 44.0)),
                    )
                    .clicked()
                {
                    self.play();
                }
                if ui
                    .add_enabled(
                        (busy || state == PlaybackState::Failed) && !self.stopping,
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
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add_sized([120.0, 38.0], egui::Button::new("SETTINGS ▾"))
                    .clicked()
                {
                    self.settings_open = !self.settings_open;
                    if self.settings_open {
                        self.refresh_outputs();
                    }
                }
                ui.add(egui::Label::new(egui::RichText::new(&self.note).small()).truncate())
                    .on_hover_text(&self.note);
            });
            if self.settings_open {
                self.show_settings(ui.ctx());
            }
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        });
    }
}
