use super::{theme, RadioPanel, Segment, PANEL_HEIGHT, PANEL_WIDTH};
use eframe::egui;
use rust_radio_audio::compat_controller::PlaybackState;

impl RadioPanel {
    /// Render controls and poll owned workers. Opt-in discovery queues background
    /// scans; receiver, network and audio work stays off the render thread.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.poll_shutdown();
        self.maybe_scan_inputs();
        let frame = egui::Frame::new()
            .fill(theme::SCREEN)
            .stroke(egui::Stroke::new(1.0_f32, theme::GREEN))
            .inner_margin(egui::Margin::same(9));
        frame.show(ui, |ui| {
            ui.set_max_width((ui.available_width() - 2.0).min(PANEL_WIDTH - 20.0));
            ui.set_min_height((ui.available_height() - 2.0).min(PANEL_HEIGHT - 20.0));
            theme::apply(ui);
            theme::scanlines(ui);
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

            if running && self.segment == Segment::Ip && !self.stopping && !self.restart_pending {
                self.note = match state {
                    PlaybackState::Connecting => "CONNECTING / REQUESTING STREAM".into(),
                    PlaybackState::Buffering => "BUFFERING / WAITING FOR AUDIO OUTPUT".into(),
                    PlaybackState::Playing => "PLAYING / DECODED AUDIO REACHED OUTPUT".into(),
                    PlaybackState::Recovering => "RECONNECTING / RETRYING STREAM".into(),
                    _ => self.note.clone(),
                };
            }

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                ui.spacing_mut().button_padding = egui::vec2(5.0, 2.0);
                for (index, segment) in [Segment::Ip, Segment::Network, Segment::Usb]
                    .into_iter()
                    .enumerate()
                {
                    if index > 0 {
                        ui.colored_label(egui::Color32::from_rgb(45, 75, 49), "|");
                    }
                    let available = self.segment_available(segment);
                    if theme::input_button(ui, segment.label(), available, self.segment == segment)
                        .on_hover_text(if available {
                            "Available — click for setup"
                        } else {
                            "Unavailable or not checked — click for setup and scan"
                        })
                        .clicked()
                    {
                        self.select_segment(segment, true);
                    }
                }
                ui.add_space(6.0);
                ui.label(if self.restart_pending {
                    "CONNECTING".into()
                } else if self.stopping {
                    "STOPPING".into()
                } else {
                    if self.validation.is_some() {
                        "CONNECTING".into()
                    } else {
                        format!("{:?}", state).to_uppercase()
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("RUST RADIO");
                });
            });
            ui.separator();

            self.station_selector(ui);
            self.cancel_stale_validation();
            ui.add(egui::Label::new(self.source_detail()).truncate());
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !running
                            && !self.restart_pending
                            && !self.stopping
                            && self.segment_supported(),
                        egui::Button::new(if state == PlaybackState::Failed {
                            "↻  RETRY"
                        } else {
                            "▶  PLAY"
                        })
                        .min_size(egui::vec2(116.0, 44.0)),
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
            let failed = state == PlaybackState::Failed;
            let detail = if failed {
                format!("! {}", self.note)
            } else {
                self.note.clone()
            };
            egui::ScrollArea::vertical()
                .id_salt("radio_status_detail")
                .max_height(52.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(egui::RichText::new(detail).small().color(if failed {
                            theme::ERROR
                        } else {
                            theme::GREEN
                        }))
                        .wrap(),
                    );
                });
            if self.settings_open {
                self.show_settings(ui.ctx());
            }
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        });
    }
}
