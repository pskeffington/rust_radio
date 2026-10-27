use super::RadioPanel;
use eframe::egui;

impl RadioPanel {
    pub(super) fn show_settings(&mut self, ctx: &egui::Context) -> Option<egui::Rect> {
        egui::Window::new("RADIO / SETTINGS")
            .id(egui::Id::new("rust_radio_output_overlay"))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .fixed_size(egui::vec2(380.0, 230.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label("Stop playback before selecting another output.");
                ui.add_enabled_ui(self.engine.is_none(), |ui| {
                    egui::ComboBox::from_id_salt("rust_radio_output")
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
                        self.settings_open = false;
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if let Some(task) = &self.validation {
                        if ui
                            .add_enabled(
                                !task.cancellation_requested(),
                                egui::Button::new("CANCEL CHECK"),
                            )
                            .clicked()
                        {
                            task.request_cancel();
                        }
                    } else if ui
                        .add_enabled(self.engine.is_none(), egui::Button::new("CHECK STATION"))
                        .clicked()
                    {
                        self.start_validation();
                    }
                    ui.label(self.validation_label(self.selected_station().id));
                });
                if let Some(report) = self.validations.get(self.selected_station().id) {
                    ui.add(egui::Label::new(&report.detail).truncate())
                        .on_hover_text(&report.detail);
                    let age = report
                        .checked_at
                        .and_then(|time| time.elapsed().ok())
                        .map(|elapsed| format!("Checked {}s ago", elapsed.as_secs()))
                        .unwrap_or_else(|| "Check time unavailable".into());
                    let format = match (&report.codec, report.sample_rate, report.channels) {
                        (Some(codec), Some(rate), Some(channels)) => {
                            format!(" / {codec} / {rate} Hz / {channels} ch")
                        }
                        _ => String::new(),
                    };
                    ui.add(egui::Label::new(format!("{age}{format}")).truncate());
                } else {
                    ui.label("Checks decode silently; availability can change.");
                }
            })
            .map(|response| response.response.rect)
    }
}
