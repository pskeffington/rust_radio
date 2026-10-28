use super::*;
use eframe::egui;
use rust_radio_audio::compat_controller::PlaybackState;
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
            panel.selected_station().id,
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
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(PANEL_WIDTH, PANEL_HEIGHT));
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
    assert_ne!(panel.selected_station().id, "skylab");
    assert!(panel.validation.as_ref().unwrap().cancellation_requested());
    assert_eq!(
        panel.validation_label(panel.selected_station().id),
        "UNVERIFIED"
    );
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
    assert_eq!(
        panel.validation_label(panel.selected_station().id),
        "UNVERIFIED"
    );
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
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(PANEL_WIDTH, PANEL_HEIGHT));
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
        panel.validations.insert("skylab", report);
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
    assert_eq!(panel.selected_station().id, "skylab");
}

#[test]
fn panel_skin_does_not_change_the_host_theme() {
    let mut panel = RadioPanel::new();
    panel.settings_open = true;
    let context = egui::Context::default();
    let original = context.style().visuals.clone();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(PANEL_WIDTH, PANEL_HEIGHT));
    let _ = context.run(
        egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                panel.show(ui);
            });
        },
    );
    let after = &context.style().visuals;
    assert_eq!(after.override_text_color, original.override_text_color);
    assert_eq!(after.window_fill, original.window_fill);
    assert_eq!(
        after.widgets.inactive.bg_fill,
        original.widgets.inactive.bg_fill
    );
}
#[test]
fn changing_station_is_offline() {
    let mut panel = RadioPanel::new();
    let original = panel.station_index;
    panel.step_station(1);
    assert_ne!(original, panel.station_index);
    assert!(!panel.is_active());
}

#[test]
fn station_indexes_preserve_catalog_order_and_wrap_in_both_directions() {
    let mut panel = RadioPanel::new();
    let stations = curated_stations();
    let count = stations.len()
        + usize::from(cfg!(feature = "sdrconnect"))
        + usize::from(cfg!(feature = "soapy-sdr"));
    for index in 0..count {
        assert_eq!(panel.source_index(), index);
        if index < stations.len() {
            assert_eq!(panel.selected_station(), &stations[index]);
        }
        panel.step_station(1);
    }
    assert_eq!(panel.source_index(), 0);
    panel.step_station(-1);
    assert_eq!(panel.source_index(), count - 1);
    panel.step_station(1);
    assert_eq!(panel.source_index(), 0);
    assert!(!panel.is_active());
}

#[test]
fn large_station_steps_remain_in_bounds_without_overflow() {
    let mut panel = RadioPanel::new();
    let count = curated_stations().len()
        + usize::from(cfg!(feature = "sdrconnect"))
        + usize::from(cfg!(feature = "soapy-sdr"));
    for direction in [isize::MIN, isize::MAX, -100, -1, 0, 1, 100] {
        panel.select_source(count - 1);
        let expected = ((count as i128 - 1) + direction as i128).rem_euclid(count as i128) as usize;
        panel.step_station(direction);
        assert_eq!(panel.source_index(), expected);
        if expected < curated_stations().len() {
            assert_eq!(panel.selected_station(), &curated_stations()[expected]);
        }
    }
    assert!(!panel.is_active());
}

#[cfg(feature = "soapy-sdr")]
#[test]
fn usb_selection_is_offline_and_invalid_configuration_does_not_start_a_worker() {
    let mut panel = RadioPanel::new();
    panel.select_source(curated_stations().len() + usize::from(cfg!(feature = "sdrconnect")));
    assert!(panel.is_usb_selected());
    panel.start_validation();
    assert!(panel.validation.is_none());
    panel.play();
    assert!(!panel.is_active());
    assert!(panel.note.contains("discovered SoapySDR receiver ID"));
    panel.usb_id = "soapy-0000000000000000".into();
    panel.usb_frequency = "invalid".into();
    panel.play();
    assert!(!panel.is_active());
    assert!(panel.note.contains("frequency"));
    render_panel(&mut panel);
    let context = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(PANEL_WIDTH, PANEL_HEIGHT));
    let mut bounds = None;
    for _ in 0..2 {
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                bounds = panel.show_settings(ctx);
            },
        );
    }
    assert!(screen.contains_rect(bounds.unwrap()));
}

#[cfg(feature = "sdrconnect")]
#[test]
fn sdr_selection_is_offline_and_invalid_endpoint_fails_before_start() {
    let mut panel = RadioPanel::new();
    panel.select_source(curated_stations().len());
    assert!(panel.is_sdr_selected());
    assert_eq!(panel.sdr_endpoint, "127.0.0.1:5454");
    assert!(!panel.is_active());
    panel.start_validation();
    assert!(panel.validation.is_none());
    panel.last_session = Some(RadioSession {
        state: PlaybackState::Failed,
        last_error: Some("Earlier receiver failure".into()),
        ..Default::default()
    });
    panel.sdr_endpoint = "8.8.8.8:5454".into();
    panel.play();
    assert!(!panel.is_active());
    assert!(panel.last_session.is_none());
    assert!(panel.note.contains("private-LAN or loopback"));
    render_panel(&mut panel);
    assert!(panel.note.contains("private-LAN or loopback"));
    let context = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(PANEL_WIDTH, PANEL_HEIGHT));
    let mut bounds = None;
    for _ in 0..2 {
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                bounds = panel.show_settings(ctx);
            },
        );
    }
    assert!(screen.contains_rect(bounds.unwrap()));
}

#[cfg(feature = "sdrconnect")]
#[test]
fn selecting_sdr_cancels_an_in_flight_station_check() {
    let mut panel = RadioPanel::new();
    let release = hold_check(&mut panel);
    panel.select_source(curated_stations().len());
    assert!(panel.validation.as_ref().unwrap().cancellation_requested());
    release.send(()).unwrap();
    finish_panel(&mut panel);
    assert!(panel.validations.is_empty());
    assert!(panel.is_sdr_selected());
}
