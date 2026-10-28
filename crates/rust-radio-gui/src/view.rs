use super::{theme, RadioPanel, Segment, PANEL_HEIGHT, PANEL_WIDTH};
use eframe::egui;

impl RadioPanel {
    /// Compact listener controls. Workers and storage stay owned by the host.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.poll_shutdown();
        self.maybe_scan_inputs();
        egui::Frame::new()
            .fill(theme::SCREEN)
            .stroke(egui::Stroke::new(1.0_f32, theme::BORDER))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.set_max_width((ui.available_width() - 2.0).min(PANEL_WIDTH - 26.0));
                ui.set_min_height((ui.available_height() - 2.0).min(PANEL_HEIGHT - 26.0));
                theme::apply(ui);
                theme::scanlines(ui);
                ui.spacing_mut().item_spacing = egui::vec2(7.0, 6.0);
                ui.spacing_mut().button_padding = egui::vec2(10.0, 8.0);
                self.input_header(ui);
                ui.separator();
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(if self.segment == Segment::Ip {
                        "STATION"
                    } else {
                        "RECEIVER"
                    })
                    .small()
                    .color(theme::MUTED),
                );
                self.station_selector(ui);
                self.cancel_stale_validation();
                ui.add(
                    egui::Label::new(egui::RichText::new(self.source_detail()).color(theme::MUTED))
                        .truncate(),
                )
                .on_hover_text(self.source_detail());
                ui.add_space(8.0);

                let listening = self.listening_status();
                self.playback_controls(ui, &listening);
                ui.add_space(6.0);
                self.volume_controls(ui);
                ui.add_space(2.0);
                self.listening_feedback(ui, &listening);
                if self.settings_open {
                    self.show_settings(ui.ctx());
                }
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100));
            });
    }
    fn playback_controls(
        &mut self,
        ui: &mut egui::Ui,
        status: &super::presentation::ListeningStatus,
    ) {
        ui.horizontal(|ui| {
            let label = if status.stop_action {
                "■  STOP"
            } else if status.failed {
                "RETRY"
            } else {
                "▶  PLAY"
            };
            let enabled = if status.stop_action {
                !self.stopping || self.restart_pending
            } else {
                self.engine.is_none() && !self.stopping && self.segment_supported()
            };
            let action = egui::Button::new(egui::RichText::new(label).strong())
                .fill(theme::ACTION)
                .min_size(egui::vec2(122.0, 46.0));
            if ui.add_enabled(enabled, action).clicked() {
                if status.stop_action {
                    self.stop();
                } else {
                    self.play();
                }
            }
            ui.add_space(5.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if status.working {
                        ui.add(egui::Spinner::new().size(14.0));
                    } else {
                        let color = if status.failed {
                            theme::ERROR
                        } else if status.label == "Playing" {
                            theme::GREEN
                        } else {
                            theme::MUTED
                        };
                        ui.colored_label(color, if status.failed { "!" } else { "●" });
                    }
                    ui.label(egui::RichText::new(status.label).size(18.0).strong().color(
                        if status.failed {
                            theme::ERROR
                        } else {
                            theme::GREEN
                        },
                    ));
                });
                if self.validation.is_some()
                    && !status.stop_action
                    && ui.small_button("Cancel check").clicked()
                {
                    self.stop();
                }
            });
        });
    }

    fn volume_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().slider_width = (ui.available_width() - 176.0).max(100.0);
            let mut value = self.volume;
            let slider = egui::Slider::new(&mut value, 0..=100)
                .show_value(false)
                .text("Volume");
            if ui.add(slider).on_hover_text("Volume").changed() {
                self.change_volume(value);
            }
            ui.label(format!("{:>3}%", self.volume));
            let mute = egui::Button::new(if self.volume == 0 { "UNMUTE" } else { "MUTE" })
                .selected(self.volume == 0)
                .min_size(egui::vec2(66.0, 30.0));
            if ui.add(mute).clicked() {
                self.toggle_mute();
            }
        });
    }

    fn listening_feedback(&self, ui: &mut egui::Ui, status: &super::presentation::ListeningStatus) {
        if status.failed {
            let detail = self.listener_error_detail();
            egui::ScrollArea::vertical()
                .id_salt("radio_status_detail")
                .min_scrolled_height(0.0)
                .max_height(ui.available_height().clamp(8.0, 44.0))
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(egui::RichText::new(&detail).small().color(theme::ERROR))
                            .wrap(),
                    )
                    .on_hover_text(self.playback_error().unwrap_or(detail));
                });
        } else {
            let detail = if status.label == "Playing" {
                format!("Output: {}", self.output_name())
            } else {
                status.hint.to_owned()
            };
            ui.add(
                egui::Label::new(egui::RichText::new(detail).small().color(theme::MUTED))
                    .truncate(),
            );
        }
    }

    fn input_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.spacing_mut().button_padding = egui::vec2(6.0, 3.0);
            for segment in [Segment::Ip, Segment::Network, Segment::Usb] {
                if theme::input_button(
                    ui,
                    segment.label(),
                    self.segment_available(segment),
                    self.segment == segment,
                )
                .on_hover_text(format!(
                    "{} settings · {}",
                    segment.label(),
                    if self.segment_available(segment) {
                        "available"
                    } else {
                        "not detected yet"
                    }
                ))
                .clicked()
                {
                    self.select_segment(segment, true);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new("RUST RADIO")
                        .small()
                        .color(theme::MUTED),
                );
            });
        });
    }
}
