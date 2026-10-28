use super::{RadioPanel, Segment};
use eframe::egui;
use rust_radio_audio::compat_controller::PlaybackState;
impl RadioPanel {
    pub(super) fn show_ip_settings(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Audio output");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("CLOSE").clicked() {
                    self.settings_open = false;
                }
            });
        });
        ui.add_enabled_ui(self.engine.is_none() && !self.restart_pending, |ui| {
            egui::ComboBox::from_id_salt("ip_output")
                .width(325.0)
                .truncate()
                .selected_text(self.output_name())
                .show_ui(ui, |ui| {
                    ui.set_max_width(325.0);
                    ui.selectable_value(&mut self.output, "system".into(), "System default");
                    for device in &self.outputs {
                        ui.add_enabled_ui(device.available, |ui| {
                            ui.selectable_value(&mut self.output, device.id.clone(), &device.name);
                        });
                    }
                });
        })
        .response
        .on_hover_text("Stop playback to change audio output");
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("ip_setup_body")
            .max_height(150.0)
            .show(ui, |ui| {
                if self.direct_selected {
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ip_station_name)
                                .desired_width(265.0)
                                .char_limit(64),
                        );
                    });
                    ui.label("Stream URL")
                        .on_hover_text("Public HTTPS or private IPv4 HTTP; MP3, AAC, M3U and PLS");
                    let edited = ui
                        .add(
                            egui::TextEdit::singleline(&mut self.direct_url)
                                .desired_width(325.0)
                                .char_limit(2048)
                                .hint_text("https://… or http://192.168.…"),
                        )
                        .changed();
                    if edited {
                        self.saved_station_index = None;
                        if self.engine.is_none() {
                            self.ip_selection_changed();
                        } else {
                            self.note = "URL changed. Choose Apply stream to reconnect.".into();
                        }
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .button("SAVE STATION")
                            .on_hover_text("Save the name and complete URL on this device")
                            .clicked()
                        {
                            self.note = match self.save_ip_station() {
                                Ok(()) => "Station added to your saved stations.".into(),
                                Err(error) => error,
                            };
                        }
                        if ui
                            .add_enabled(
                                self.saved_station_index.is_some(),
                                egui::Button::new("REMOVE"),
                            )
                            .clicked()
                        {
                            if let Some(index) = self.saved_station_index.take() {
                                self.ip_stations.remove(index);
                                self.ip_stations_dirty = true;
                                self.note =
                                    "Station removed. The URL is still available to edit.".into();
                            }
                        }
                    });
                } else {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(self.selected_station().name).strong(),
                        )
                        .truncate(),
                    );
                    ui.label(self.stream_check_summary());
                    if ui.button("ADD STATION URL").clicked() {
                        self.select_ip_station(self.ip_station_count() - 1);
                    }
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.stopping,
                            egui::Button::new(if self.engine.is_some() {
                                "APPLY STREAM"
                            } else {
                                "CHECK STREAM"
                            }),
                        )
                        .clicked()
                        && self.segment == Segment::Ip
                    {
                        self.ip_selection_changed();
                        if self.engine.is_none() {
                            self.selection_check_pending = Some(std::time::Instant::now());
                        }
                    }
                    if ui.button("FIND OUTPUTS").clicked() {
                        self.refresh_outputs();
                    }
                });
                if [
                    "Station ",
                    "Cannot ",
                    "URL changed.",
                    "Invalid ",
                    "Stream URL",
                    "Use an ",
                    "HTTP streams",
                ]
                .iter()
                .any(|prefix| self.note.starts_with(prefix))
                {
                    ui.add(egui::Label::new(egui::RichText::new(&self.note).small()).wrap());
                }
                egui::CollapsingHeader::new("Connection details").show(ui, |ui| {
                    ui.label(self.stream_check_summary());
                    let report = if self.direct_selected {
                        self.direct_validation.as_ref()
                    } else {
                        self.validations.get(self.selected_station().id)
                    };
                    if let Some(report) = report {
                        ui.label(format!("Last silent check: {}", report.detail));
                    }
                    ui.label(self.playback_error().unwrap_or_else(|| self.note.clone()));
                });
            });
    }
    pub(super) fn stream_check_summary(&self) -> &'static str {
        if self
            .engine
            .as_ref()
            .is_some_and(|engine| engine.status().state == PlaybackState::Playing)
        {
            "Receiving audio now"
        } else if self.validation.is_some() {
            "Checking the stream…"
        } else {
            let report = if self.direct_selected {
                self.direct_validation.as_ref()
            } else {
                self.validations.get(self.selected_station().id)
            };
            if report.is_some_and(|report| report.is_decoded()) {
                "Stream check passed. Press Play to listen."
            } else {
                "The stream is checked when you press Play."
            }
        }
    }
}
