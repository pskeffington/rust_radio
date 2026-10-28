use super::{theme, RadioPanel, Segment};
use eframe::egui;

impl RadioPanel {
    pub(super) fn show_settings(&mut self, ctx: &egui::Context) -> Option<egui::Rect> {
        egui::Window::new(
            egui::RichText::new(format!("{} / SETUP", self.segment.label()))
                .monospace()
                .color(theme::GREEN),
        )
        .id(egui::Id::new("rust_radio_output_overlay"))
        .frame(
            egui::Frame::new()
                .fill(theme::SCREEN)
                .stroke(egui::Stroke::new(1.0_f32, theme::GREEN))
                .inner_margin(egui::Margin::same(8)),
        )
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .fixed_size(egui::vec2(380.0, 230.0))
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            theme::apply(ui);
            #[cfg(feature = "nrsp-native")]
            if self.native_selected {
                self.show_native_settings(ui);
                return;
            }
            if self.segment == Segment::Ip {
                self.show_ip_settings(ui);
                return;
            }
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
                if ui
                    .add_enabled(
                        self.input_scan.is_none() && self.engine.is_none(),
                        egui::Button::new(if self.input_scan.is_some() {
                            "SCANNING…"
                        } else {
                            "SCAN INPUTS"
                        }),
                    )
                    .clicked()
                {
                    self.scan_inputs();
                }
                ui.label(match self.segment {
                    Segment::Ip => {
                        if self.inputs.internet {
                            "INTERNET AVAILABLE"
                        } else {
                            "INTERNET NOT VERIFIED"
                        }
                    }
                    Segment::Network => {
                        if self.inputs.network_available() {
                            "RECEIVER FOUND"
                        } else {
                            "NO READY NETWORK RECEIVER"
                        }
                    }
                    Segment::Usb => {
                        if self.inputs.usb_available() {
                            "RECEIVER FOUND"
                        } else {
                            "NO USB RECEIVER"
                        }
                    }
                });
            });
            if !self.segment_supported() {
                ui.label(match self.segment {
                    Segment::Network => &self.inputs.network_note,
                    Segment::Usb => &self.inputs.usb_note,
                    Segment::Ip => "",
                });
            }
            #[cfg(feature = "soapy-sdr")]
            if self.is_usb_selected() {
                let enabled = !self.is_active();
                ui.add_enabled_ui(enabled, |ui| {
                    egui::ComboBox::from_id_salt("rust_radio_usb_found")
                        .width(320.0)
                        .selected_text(
                            self.inputs
                                .usb
                                .iter()
                                .find(|device| device.id == self.usb_id)
                                .map(|device| device.label.as_str())
                                .unwrap_or("No USB receiver found"),
                        )
                        .show_ui(ui, |ui| {
                            for device in &self.inputs.usb {
                                ui.selectable_value(
                                    &mut self.usb_id,
                                    device.id.clone(),
                                    &device.label,
                                );
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("FM FREQ Hz");
                    ui.add_enabled(
                        enabled,
                        egui::TextEdit::singleline(&mut self.usb_frequency)
                            .desired_width(190.0)
                            .char_limit(12),
                    );
                });
                ui.label("Scan after attaching a receiver / mono FM.");
            }
            #[cfg(feature = "sdrconnect")]
            if self.is_sdr_selected() {
                ui.label("SDRconnect API (IP:port)");
                ui.add_enabled(
                    !self.is_active(),
                    egui::TextEdit::singleline(&mut self.sdr_endpoint)
                        .desired_width(325.0)
                        .char_limit(80),
                );
                ui.label("Refresh and select the receiver in SDRconnect.");
            }
        })
        .map(|response| response.response.rect)
    }
}
