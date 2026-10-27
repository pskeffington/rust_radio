//! Embeddable 480x320 radio surface. The host owns the window and event loop.
//! Constructing a panel never opens an audio device or connects to a broadcaster.
use eframe::egui;
use rust_radio_audio::compat_controller::{PlaybackState, RadioSession};
use rust_radio_audio::output::{available_outputs, OutputDeviceInfo};
use rust_radio_audio::playback::NativeRadioEngine;
use rust_radio_audio::validation::StationValidationTask;
use rust_radio_core::directory::{curated_stations, station_by_id};
use rust_radio_core::validation::{StationValidation, ValidationStatus};
use std::collections::BTreeMap;

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
    last_session: Option<RadioSession>,
    validation: Option<StationValidationTask>,
    validations: BTreeMap<String, StationValidation>,
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
            last_session: None,
            validation: None,
            validations: BTreeMap::new(),
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
        let active = self.is_active();
        if let Some(engine) = &self.engine {
            engine.request_stop();
            self.stopping = true;
            self.note = "STOPPING / RELEASING RECEIVER".into();
        }
        if let Some(task) = &self.validation {
            task.request_cancel();
            self.note = "CANCELLING CHECK / RELEASING NETWORK".into();
        }
        if !active {
            self.last_session = None;
            self.note = "RECEIVER STOPPED".into();
        }
        self.poll_shutdown();
    }

    /// Reap finished receivers even while this panel is hidden. False means
    /// a worker still owns resources. A completed failure remains visible until
    /// the operator explicitly starts another session or clears it with Stop.
    pub fn poll_shutdown(&mut self) -> bool {
        if let Some(engine) = self.engine.as_mut() {
            if engine.try_finish() {
                let session = engine.status();
                self.engine = None;
                self.stopping = false;
                self.note = session
                    .last_error
                    .clone()
                    .unwrap_or_else(|| "RECEIVER STOPPED".into());
                self.last_session = Some(session);
            }
        }
        self.cancel_stale_validation();
        if let Some(task) = self.validation.as_mut() {
            if let Some(result) = task.try_finish() {
                self.validation = None;
                match result {
                    Ok(report) if report.status == ValidationStatus::Cancelled => {
                        self.note = "STATION CHECK CANCELLED".into();
                    }
                    Ok(report) => {
                        let id = report.station_id.clone();
                        self.validations.insert(id.clone(), report);
                        self.note = format!("STATION CHECK / {}", self.validation_label(&id));
                    }
                    Err(error) => self.note = error,
                }
            }
        }
        !self.is_active()
    }

    pub fn is_active(&self) -> bool {
        self.engine.is_some() || self.validation.is_some()
    }

    fn play(&mut self) {
        if self.is_active() {
            return;
        }
        if station_by_id(&self.station).is_none() {
            self.note = "UNKNOWN STATION".into();
            return;
        }
        match NativeRadioEngine::start_internet(&self.station, self.volume, &self.output) {
            Ok(engine) => {
                self.engine = Some(engine);
                self.last_session = None;
                self.note = "CONNECTING / NATIVE AUDIO".into();
            }
            Err(error) => self.note = error,
        }
    }

    fn start_validation(&mut self) {
        if self.is_active() {
            return;
        }
        match StationValidationTask::start(&self.station) {
            Ok(task) => {
                self.validation = Some(task);
                self.note = "CHECKING STATION / NO AUDIO OUTPUT".into();
            }
            Err(error) => self.note = error,
        }
    }

    fn cancel_stale_validation(&mut self) {
        if let Some(task) = &self.validation {
            if task.station_id() != self.station {
                task.request_cancel();
            }
        }
    }

    fn validation_label(&self, id: &str) -> &'static str {
        if let Some(task) = &self.validation {
            if task.station_id() == id {
                return if task.cancellation_requested() {
                    "CANCELLING"
                } else {
                    "CHECKING"
                };
            }
        }
        match self.validations.get(id) {
            Some(report) if report.is_decoded() => "DECODED",
            Some(report) => match report.status {
                ValidationStatus::Unavailable => "UNAVAILABLE",
                ValidationStatus::InvalidStream => "INVALID STREAM",
                ValidationStatus::Unsupported => "UNSUPPORTED",
                _ => "UNVERIFIED",
            },
            None => "UNVERIFIED",
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
                        .selected_text(
                            station_by_id(&self.station)
                                .map(|s| s.name)
                                .unwrap_or("Select station"),
                        )
                        .show_ui(ui, |ui| {
                            for station in curated_stations() {
                                let label = format!(
                                    "{} [{}]",
                                    station.name,
                                    self.validation_label(station.id)
                                );
                                ui.selectable_value(
                                    &mut self.station,
                                    station.id.to_owned(),
                                    label,
                                );
                            }
                        });
                    if ui.add_sized([45.0, 43.0], egui::Button::new("▶")).clicked() {
                        self.step_station(1);
                    }
                });
            });
            self.cancel_stale_validation();
            if let Some(station) = station_by_id(&self.station) {
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
                    self.show_output = !self.show_output;
                    if self.show_output {
                        self.refresh_outputs();
                    }
                }
                ui.add(egui::Label::new(egui::RichText::new(&self.note).small()).truncate())
                    .on_hover_text(&self.note);
            });
            if self.show_output {
                self.show_settings(ui.ctx());
            }
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        });
    }

    fn show_settings(&mut self, ctx: &egui::Context) -> Option<egui::Rect> {
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
                        self.show_output = false;
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
                    ui.label(self.validation_label(&self.station));
                });
                if let Some(report) = self.validations.get(&self.station) {
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

    fn step_station(&mut self, direction: isize) {
        let list = curated_stations();
        if list.is_empty() {
            return;
        }
        let index = list.iter().position(|s| s.id == self.station).unwrap_or(0) as isize;
        let next = (index + direction).rem_euclid(list.len() as isize) as usize;
        self.station = list[next].id.into();
        self.cancel_stale_validation();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_radio_core::stream::{ApprovedEndpoint, HttpResponse, RadioHttp, StreamError};
    use std::io::Cursor;
    use std::sync::{mpsc, Mutex};
    use std::time::{Duration, Instant, SystemTime};

    struct GatedHttp {
        entered: mpsc::Sender<()>,
        release: Mutex<mpsc::Receiver<()>>,
    }

    impl RadioHttp for GatedHttp {
        type Body = Cursor<Vec<u8>>;

        fn get(&self, _: &ApprovedEndpoint) -> Result<HttpResponse<Self::Body>, StreamError> {
            self.entered.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            Ok(HttpResponse {
                status: 200,
                content_type: Some("audio/mpeg".into()),
                redirect_location: None,
                icy_metaint: None,
                body: Cursor::new(
                    include_bytes!("../../rust-radio-audio/tests/fixtures/tone.mp3").to_vec(),
                ),
            })
        }
    }

    fn hold_check(panel: &mut RadioPanel) -> mpsc::Sender<()> {
        let (entered, ready) = mpsc::channel();
        let (release, pending) = mpsc::channel();
        panel.validation = Some(
            StationValidationTask::with_transport(
                GatedHttp {
                    entered,
                    release: Mutex::new(pending),
                },
                &panel.station,
            )
            .unwrap(),
        );
        ready.recv_timeout(Duration::from_secs(3)).unwrap();
        release
    }

    fn finish_panel(panel: &mut RadioPanel) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while !panel.poll_shutdown() {
            assert!(Instant::now() < deadline, "Panel worker did not finish");
            std::thread::yield_now();
        }
    }

    fn render_panel(panel: &mut RadioPanel) -> egui::Rect {
        let context = egui::Context::default();
        let screen =
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(PANEL_WIDTH, PANEL_HEIGHT));
        let mut bounds = egui::Rect::NOTHING;
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    panel.show(ui);
                    bounds = ui.min_rect();
                });
            },
        );
        assert!(
            screen.contains_rect(bounds),
            "Panel exceeds viewport: {bounds:?}"
        );
        bounds
    }

    #[test]
    fn pending_check_keeps_rendering_and_blocks_duplicate_checks_and_playback() {
        let mut panel = RadioPanel::new();
        let release = hold_check(&mut panel);
        assert_eq!(panel.validation_label("skylab"), "CHECKING");
        assert!(!panel.poll_shutdown());
        panel.start_validation();
        panel.play();
        assert!(panel.engine.is_none());
        assert!(panel.validation.is_some());
        render_panel(&mut panel);
        panel.stop();
        assert!(panel.is_active());
        assert_eq!(panel.validation_label("skylab"), "CANCELLING");
        assert!(!panel.poll_shutdown());
        panel.stop();
        release.send(()).unwrap();
        finish_panel(&mut panel);
        assert!(panel.validations.is_empty());
        assert_eq!(panel.validation_label("skylab"), "UNVERIFIED");
    }

    #[test]
    fn changing_station_cancels_and_discards_the_previous_check() {
        let mut panel = RadioPanel::new();
        let release = hold_check(&mut panel);
        panel.step_station(1);
        assert_ne!(panel.station, "skylab");
        assert!(panel.validation.as_ref().unwrap().cancellation_requested());
        assert_eq!(panel.validation_label(&panel.station), "UNVERIFIED");
        assert!(!panel.poll_shutdown());
        release.send(()).unwrap();
        finish_panel(&mut panel);
        assert!(panel.validations.is_empty());
    }

    #[test]
    fn finished_checks_belong_to_the_checked_station_and_cancel_preserves_evidence() {
        let mut panel = RadioPanel::new();
        let release = hold_check(&mut panel);
        release.send(()).unwrap();
        finish_panel(&mut panel);
        assert_eq!(panel.validation_label("skylab"), "DECODED");
        assert!(panel.validations["skylab"].is_decoded());
        let evidence = panel.validations["skylab"].clone();
        panel.step_station(1);
        assert_eq!(panel.validation_label(&panel.station), "UNVERIFIED");
        panel.step_station(-1);
        let release = hold_check(&mut panel);
        panel.stop();
        release.send(()).unwrap();
        finish_panel(&mut panel);
        assert_eq!(panel.validations["skylab"], evidence);
        assert_eq!(panel.validation_label("skylab"), "DECODED");
    }

    #[test]
    fn station_status_and_long_diagnostics_fit_the_micro_viewport() {
        let mut panel = RadioPanel::new();
        let context = egui::Context::default();
        let screen =
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(PANEL_WIDTH, PANEL_HEIGHT));
        for status in [
            ValidationStatus::Unavailable,
            ValidationStatus::Unsupported,
            ValidationStatus::InvalidStream,
        ] {
            let mut report = StationValidation::unverified("skylab");
            report.status = status;
            report.checked_at = Some(SystemTime::now());
            report.detail = "Long diagnostic text ".repeat(100);
            panel.note = report.detail.clone();
            panel.validations.insert("skylab".into(), report);
            render_panel(&mut panel);
            let mut settings = None;
            // egui measures a new Window before its final anchored placement.
            for _ in 0..2 {
                let _ = context.run(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |ctx| {
                        settings = panel.show_settings(ctx);
                    },
                );
            }
            let bounds = settings.unwrap();
            assert!(
                screen.contains_rect(bounds),
                "Settings exceed viewport: {bounds:?}"
            );
            assert!(!panel.is_active());
        }
    }

    #[test]
    fn completed_failure_stays_visible_without_retaining_an_active_receiver() {
        let mut panel = RadioPanel::new();
        panel.last_session = Some(RadioSession {
            state: PlaybackState::Failed,
            last_error: Some("Speaker disconnected".into()),
            ..Default::default()
        });
        assert!(panel.poll_shutdown());
        assert!(!panel.is_active());
        let session = panel.last_session.as_ref().unwrap();
        assert_eq!(session.state, PlaybackState::Failed);
        assert_eq!(session.last_error.as_deref(), Some("Speaker disconnected"));
        panel.stop();
        assert!(panel.last_session.is_none());
        assert_eq!(panel.note, "RECEIVER STOPPED");
    }

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
