use super::{theme, RadioPanel, Segment, PANEL_HEIGHT, PANEL_WIDTH};
use eframe::egui;
use rust_radio_audio::compat_controller::PlaybackState;
use rust_radio_core::directory::curated_stations;

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
                    if ui
                        .add_enabled(
                            self.segment == Segment::Ip,
                            egui::Button::new("◀").min_size(egui::vec2(45.0, 43.0)),
                        )
                        .clicked()
                    {
                        self.step_segment_source(-1);
                    }
                    egui::ComboBox::from_id_salt("rust_radio_station")
                        .width(260.0)
                        .truncate()
                        .selected_text(if !self.segment_supported() {
                            self.segment.label()
                        } else if self.is_native_selected() {
                            "nRSP-ST / Direct"
                        } else if self.direct_selected {
                            "Direct IP stream"
                        } else if self.is_usb_selected() {
                            "USB SDR / mono FM"
                        } else if self.is_sdr_selected() {
                            "nRSP-ST / SDRconnect"
                        } else {
                            self.selected_station().name
                        })
                        .show_ui(ui, |ui| {
                            for (index, station) in curated_stations().iter().enumerate().filter({
                                let is_ip = self.segment == Segment::Ip;
                                move |_| is_ip
                            }) {
                                let label = format!(
                                    "{} [{}]",
                                    station.name,
                                    self.validation_label(station.id)
                                );
                                if ui
                                    .selectable_label(self.source_index() == index, label)
                                    .clicked()
                                {
                                    self.select_source(index);
                                }
                            }
                            if ui
                                .add_visible(
                                    self.segment == Segment::Ip,
                                    egui::Button::new("Direct IP stream")
                                        .selected(self.direct_selected),
                                )
                                .clicked()
                            {
                                self.select_source(
                                    curated_stations().len()
                                        + usize::from(cfg!(feature = "sdrconnect"))
                                        + usize::from(cfg!(feature = "soapy-sdr")),
                                );
                            }
                            #[cfg(feature = "nrsp-native")]
                            if self.segment == Segment::Network
                                && ui
                                    .selectable_label(self.is_native_selected(), "nRSP-ST / Direct")
                                    .clicked()
                            {
                                self.select_source(
                                    curated_stations().len()
                                        + usize::from(cfg!(feature = "sdrconnect"))
                                        + usize::from(cfg!(feature = "soapy-sdr"))
                                        + 1,
                                );
                            }
                            #[cfg(feature = "sdrconnect")]
                            if self.segment == Segment::Network
                                && ui
                                    .selectable_label(
                                        self.is_sdr_selected(),
                                        "nRSP-ST / SDRconnect",
                                    )
                                    .clicked()
                            {
                                self.select_source(curated_stations().len());
                            }
                            #[cfg(feature = "soapy-sdr")]
                            if self.segment == Segment::Usb
                                && ui
                                    .selectable_label(self.is_usb_selected(), "USB SDR / mono FM")
                                    .clicked()
                            {
                                self.select_source(
                                    curated_stations().len()
                                        + usize::from(cfg!(feature = "sdrconnect")),
                                );
                            }
                        });
                    if ui
                        .add_enabled(
                            self.segment == Segment::Ip,
                            egui::Button::new("▶").min_size(egui::vec2(45.0, 43.0)),
                        )
                        .clicked()
                    {
                        self.step_segment_source(1);
                    }
                });
            });
            self.cancel_stale_validation();
            ui.add(egui::Label::new(self.source_detail()).truncate());
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !busy && !self.stopping && self.segment_supported(),
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
