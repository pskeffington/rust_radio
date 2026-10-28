use super::{RadioPanel, Segment};
use eframe::egui;
impl RadioPanel {
    pub(super) fn show_ip_settings(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("STREAM / OUTPUT");
            if ui.button("CLOSE").clicked() {
                self.settings_open = false;
            }
        });
        ui.add_enabled_ui(self.engine.is_none() && !self.restart_pending, |ui| {
            egui::ComboBox::from_id_salt("ip_output")
                .width(325.0)
                .truncate()
                .selected_text(
                    self.outputs
                        .iter()
                        .find(|d| d.id == self.output)
                        .map(|d| d.name.as_str())
                        .unwrap_or("System default"),
                )
                .show_ui(ui, |ui| {
                    ui.set_max_width(325.0);
                    ui.selectable_value(&mut self.output, "system".into(), "System default");
                    for device in &self.outputs {
                        ui.add_enabled_ui(device.available, |ui| {
                            ui.selectable_value(&mut self.output, device.id.clone(), &device.name);
                        });
                    }
                });
        });
        ui.separator();
        if self.direct_selected {
            ui.horizontal(|ui| {
                ui.label("NAME");
                ui.add(
                    egui::TextEdit::singleline(&mut self.ip_station_name)
                        .desired_width(265.0)
                        .char_limit(64),
                );
            });
            ui.label("Public HTTPS / private IPv4 HTTP · MP3, AAC, M3U, PLS");
            let edited = ui
                .add(
                    egui::TextEdit::singleline(&mut self.direct_url)
                        .desired_width(340.0)
                        .char_limit(2048),
                )
                .changed();
            if edited {
                self.saved_station_index = None;
                if self.engine.is_none() {
                    self.ip_selection_changed();
                } else {
                    self.note = "STREAM URL EDITED / APPLY TO CONNECT".into();
                }
            }
            ui.horizontal(|ui| {
                if ui.button("SAVE STATION").clicked() {
                    self.note = match self.save_ip_station() {
                        Ok(()) => "STATION SAVED / HOST STORAGE PENDING".into(),
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
                        self.note = "SAVED STATION REMOVED / URL KEPT IN EDITOR".into();
                    }
                }
            });
            ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        "Save stores this URL locally, including query parameters.",
                    )
                    .small(),
                )
                .wrap(),
            );
        } else {
            ui.label(self.selected_station().name);
            ui.label("Selection checks the stream silently. Play connects and decodes again.");
            if ui.button("ADD PUBLIC / LAN STREAM").clicked() {
                self.select_ip_station(self.ip_station_count() - 1);
            }
        }
        ui.horizontal(|ui| {
            if ui
                .button(if self.engine.is_some() {
                    "APPLY STREAM"
                } else {
                    "CHECK"
                })
                .clicked()
                && self.segment == Segment::Ip
            {
                self.ip_selection_changed();
                if self.engine.is_none() {
                    self.selection_check_pending = Some(std::time::Instant::now());
                }
            }
            if ui.button("REFRESH OUTPUTS").clicked() {
                self.refresh_outputs();
            }
        });
    }
}
