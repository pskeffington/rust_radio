use super::*;
use eframe::egui;
use rust_radio_audio::compat_controller::PlaybackState;
use rust_radio_core::stream::{ApprovedEndpoint, HttpResponse, RadioHttp, StreamError};
use rust_radio_core::validation::{StationValidation, ValidationStage, ValidationStatus};
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
fn saved_validation_history_is_bounded_freshness_checked_and_not_live_state() {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let history = serde_json::json!({
        "schema_version": 1,
        "updated_at_unix": now,
        "stations": {
            "skylab": {
                "last_checked_at_unix": now,
                "last_status": "decoded",
                "last_stage": "decoded_audio",
                "attempts": 1,
                "last_decoded_at_unix": now,
                "consecutive_failures": 0,
                "health": "fresh"
            }
        }
    })
    .to_string();
    let mut panel = RadioPanel::new();
    assert_eq!(panel.load_validation_history(&history).unwrap(), 1);
    assert_eq!(panel.validation_label("skylab"), "SAVED / FRESH");
    assert!(panel.validations.is_empty());
    assert!(panel.engine.is_none());

    let invalid = r#"{"schema_version":1,"stations":{"unknown":{"last_checked_at_unix":1}}}"#;
    assert!(panel.load_validation_history(invalid).is_err());
    assert_eq!(panel.validation_label("skylab"), "SAVED / FRESH");

    let mut live = StationValidation::unverified("skylab");
    live.status = ValidationStatus::Decoded;
    live.stage = ValidationStage::DecodedAudio;
    live.decoded_frames = 1;
    live.checked_at = Some(SystemTime::now());
    panel.validations.insert("skylab", live);
    assert_eq!(panel.validation_label("skylab"), "FRESH / DECODED");
}

#[test]
fn saved_validation_history_expires_after_the_shared_feed_ttl() {
    let old = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .saturating_sub(rust_radio_core::validation::FEED_FRESHNESS_TTL.as_secs() + 1);
    let history = serde_json::json!({
        "schema_version": 1,
        "updated_at_unix": old,
        "stations": {
            "skylab": {
                "last_checked_at_unix": old,
                "last_status": "decoded",
                "last_stage": "decoded_audio",
                "attempts": 1,
                "last_decoded_at_unix": old,
                "consecutive_failures": 0,
                "health": "fresh"
            }
        }
    })
    .to_string();
    let mut panel = RadioPanel::new();
    panel.load_validation_history(&history).unwrap();
    assert_eq!(panel.validation_label("skylab"), "SAVED / STALE");
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
    assert_eq!(panel.validation_label("skylab"), "FRESH / DECODED");
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
    assert_eq!(panel.validation_label("skylab"), "FRESH / DECODED");
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
        + usize::from(cfg!(feature = "soapy-sdr"))
        + 1
        + usize::from(cfg!(feature = "nrsp-native"));
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
        + usize::from(cfg!(feature = "soapy-sdr"))
        + 1
        + usize::from(cfg!(feature = "nrsp-native"));
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
    assert_eq!(
        panel.last_session.as_ref().unwrap().state,
        PlaybackState::Failed
    );
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

#[test]
fn direct_selection_and_validation_are_offline() {
    let mut panel = RadioPanel::new();
    let index = curated_stations().len()
        + usize::from(cfg!(feature = "sdrconnect"))
        + usize::from(cfg!(feature = "soapy-sdr"));
    panel.select_source(index);
    assert!(panel.direct_selected);
    panel.direct_url = "http://example.com/stream".into();
    assert!(panel.start_selected().is_err());
    assert!(!panel.is_active());
    panel.select_source(0);
    assert!(!panel.direct_selected);
}

#[cfg(feature = "nrsp-native")]
#[test]
fn native_receiver_configuration_is_offline_and_fits_the_viewport() {
    let mut panel = RadioPanel::new();
    let index = curated_stations().len()
        + usize::from(cfg!(feature = "sdrconnect"))
        + usize::from(cfg!(feature = "soapy-sdr"))
        + 1;
    panel.select_source(index);
    assert!(panel.is_native_selected());
    assert!(panel.is_receiver_selected());
    panel.native_endpoint = "8.8.8.8:9001".into();
    panel.play();
    assert!(!panel.is_active());
    panel.native_endpoint = "127.0.0.1:9001".into();
    panel.native_frequency = "invalid".into();
    panel.play();
    assert!(!panel.is_active());
    panel.apply_native_tuning();
    assert!(!panel.is_active());
    render_panel(&mut panel);
    let ctx = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(PANEL_WIDTH, PANEL_HEIGHT));
    let mut bounds = None;
    for _ in 0..2 {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                bounds = panel.show_settings(ctx);
            },
        );
    }
    assert!(
        screen.contains_rect(bounds.unwrap()),
        "Native settings must fit the micro viewport"
    );
    panel.select_source(0);
    assert!(!panel.is_native_selected());
}

#[test]
fn input_glow_tracks_availability_independently_of_selection() {
    use rust_radio_audio::discovery::{InputDevice, InputReport};
    for (internet, network, usb) in [
        (false, false, true),
        (false, true, false),
        (true, false, false),
        (true, true, true),
        (false, false, false),
    ] {
        let mut panel = RadioPanel::new();
        panel.apply_inputs(InputReport {
            internet,
            network: if network {
                vec![InputDevice {
                    id: "127.0.0.1:9001".into(),
                    label: "Fixture network radio".into(),
                    ready: true,
                }]
            } else {
                vec![]
            },
            usb: if usb {
                vec![InputDevice {
                    id: "soapy-0000000000000000".into(),
                    label: "Fixture USB radio".into(),
                    ready: true,
                }]
            } else {
                vec![]
            },
            ..Default::default()
        });
        assert_eq!(panel.segment_available(Segment::Ip), internet);
        assert_eq!(panel.segment_available(Segment::Network), network);
        assert_eq!(panel.segment_available(Segment::Usb), usb);
        assert_eq!(
            panel.segment,
            if network {
                Segment::Network
            } else if usb {
                Segment::Usb
            } else {
                Segment::Ip
            }
        );
        assert!(!panel.is_active(), "Detection never starts reception");
        render_panel(&mut panel);
    }
}
#[test]
fn input_setup_buttons_preserve_explicit_choice_and_stay_offline() {
    use rust_radio_audio::discovery::InputReport;
    let mut panel = RadioPanel::new();
    panel.select_segment(Segment::Ip, false);
    panel.apply_inputs(InputReport {
        internet: true,
        ..Default::default()
    });
    assert_eq!(panel.segment, Segment::Ip);
    for segment in [Segment::Ip, Segment::Network, Segment::Usb] {
        panel.select_segment(segment, false);
        assert_eq!(panel.segment, segment);
        assert!(!panel.is_active());
        panel.settings_open = true;
        let ctx = egui::Context::default();
        let screen =
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(PANEL_WIDTH, PANEL_HEIGHT));
        let mut bounds = None;
        for _ in 0..2 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ctx| {
                    bounds = panel.show_settings(ctx);
                },
            );
        }
        assert!(
            screen.contains_rect(bounds.unwrap()),
            "{} setup must fit viewport",
            segment.label()
        );
    }
}

#[test]
fn opening_ip_setup_preserves_the_selected_direct_stream() {
    let mut panel = RadioPanel::new();
    panel.select_source(
        curated_stations().len()
            + usize::from(cfg!(feature = "sdrconnect"))
            + usize::from(cfg!(feature = "soapy-sdr")),
    );
    panel.direct_url = "https://example.org/live.aac".into();
    panel.select_segment(Segment::Ip, false);
    assert!(panel.direct_selected);
    assert_eq!(panel.direct_url, "https://example.org/live.aac");
    assert!(!panel.is_active());
}

#[cfg(feature = "nrsp-native")]
#[test]
fn advanced_fm_tabs_fit_and_presets_do_not_autoplay() {
    use crate::fm_presets::{frequency_hz, PresetBank};
    let mut panel = RadioPanel::new();
    panel.select_segment(Segment::Network, false);
    panel.native_frequency = "99.1 MHz".into();
    panel.native_step = 200_000;
    panel.step_native_frequency(true);
    assert_eq!(frequency_hz(&panel.native_frequency).unwrap(), 99_300_000);
    panel.native_preset_name = "Local FM".into();
    panel.save_native_preset();
    let json = panel.take_fm_presets_update().unwrap();
    assert_eq!(PresetBank::parse(&json).unwrap().stations.len(), 2);
    assert!(panel.take_fm_presets_update().is_none());
    let before = panel.native_presets.clone();
    assert!(panel.load_fm_presets("invalid").is_err());
    assert_eq!(panel.native_presets, before);
    panel.recall_native_preset(0);
    assert_eq!(frequency_hz(&panel.native_frequency).unwrap(), 99_500_000);
    assert_eq!(panel.native_mode, "wfm");
    assert!(!panel.is_active());
    let ctx = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(PANEL_WIDTH, PANEL_HEIGHT));
    for tab in 0..4 {
        panel.native_tab = tab;
        let mut bounds = None;
        for _ in 0..2 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ctx| {
                    bounds = panel.show_settings(ctx);
                },
            );
        }
        assert!(
            screen.contains_rect(bounds.unwrap()),
            "FM tab {tab} exceeds viewport"
        );
    }
    assert!(!panel.is_active());
}

#[cfg(feature = "nrsp-native")]
#[test]
fn long_fm_preset_names_remain_within_small_display() {
    let mut panel = RadioPanel::new();
    panel.select_segment(Segment::Network, false);
    panel.native_tab = 2;
    for index in 0..15 {
        panel.native_preset_name = format!("Long preset name number {index:02} for FM");
        panel.save_native_preset();
    }
    assert_eq!(panel.native_presets.stations.len(), 16);
    let ctx = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(PANEL_WIDTH, PANEL_HEIGHT));
    let mut bounds = None;
    for _ in 0..2 {
        let _ = ctx.run(
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
    assert!(!panel.is_active());
}

#[cfg(feature = "nrsp-native")]
#[test]
fn fm_steps_reject_band_edges_without_changing_the_dial() {
    let mut panel = RadioPanel::new();
    panel.select_segment(Segment::Network, false);
    for (frequency, up) in [("108 MHz", true), ("64 MHz", false)] {
        panel.native_frequency = frequency.into();
        panel.step_native_frequency(up);
        assert_eq!(panel.native_frequency, frequency);
        assert!(!panel.is_active());
    }
}

#[test]
fn public_and_lan_saved_stations_roundtrip_without_autoplay_or_partial_import() {
    let mut panel = RadioPanel::new();
    for (name, url) in [
        (
            "Public station with a long but readable name",
            "https://example.com/live?channel=classical",
        ),
        ("LAN music", "http://192.168.1.12:8000/live.m3u"),
    ] {
        panel.direct_selected = true;
        panel.ip_station_name = name.into();
        panel.direct_url = url.into();
        panel.save_ip_station().unwrap();
    }
    let json = panel.take_ip_stations_update().unwrap();
    assert!(panel.take_ip_stations_update().is_none());
    let mut loaded = RadioPanel::new();
    loaded.load_ip_stations(&json).unwrap();
    assert_eq!(loaded.ip_station_name(), "LAN music");
    assert_eq!(loaded.direct_url, "http://192.168.1.12:8000/live.m3u");
    assert!(!loaded.is_active());
    assert!(loaded.selection_check_pending.is_none());
    assert!(loaded.take_ip_stations_update().is_none());
    for invalid in [
        json.replace("\"version\": 1", "\"version\": 99"),
        json.replace("http://192.168.1.12:8000/live.m3u", "file:///tmp/radio"),
        "x".repeat(65537),
    ] {
        assert!(loaded.load_ip_stations(&invalid).is_err());
        assert_eq!(loaded.ip_stations.len(), 2);
        assert_eq!(loaded.ip_station_name(), "LAN music");
    }
    loaded.select_ip_station(0);
    assert!(loaded.take_ip_stations_update().unwrap().contains("skylab"));
    assert_eq!(loaded.direct_url, "http://192.168.1.12:8000/live.m3u");
}

#[test]
fn rapid_selection_checks_cancel_the_old_target_and_stop_clears_queued_work() {
    let mut panel = RadioPanel::new();
    let release = hold_check(&mut panel);
    for index in [1, 2, 0, 3, 1] {
        panel.select_ip_station(index);
        panel.ip_selection_changed();
        assert!(panel.validation.as_ref().unwrap().cancellation_requested());
        assert!(panel.selection_check_pending.is_some());
        assert!(!panel.restart_pending);
        render_panel(&mut panel);
    }
    panel.play(); // Play cancels the check and queues a single latest playback.
    assert!(panel.restart_pending);
    assert!(panel.engine.is_none());
    panel.stop(); // Stop must prevent the deferred start, too.
    assert!(!panel.restart_pending);
    assert!(panel.selection_check_pending.is_none());
    release.send(()).unwrap();
    finish_panel(&mut panel);
    assert!(panel.engine.is_none());
    assert!(panel.validations.is_empty());
}

#[test]
fn full_ip_station_bank_and_custom_setup_fit_the_small_viewport() {
    let mut panel = RadioPanel::new();
    for i in 0..24 {
        panel.direct_selected = true;
        panel.direct_url = format!("http://192.168.1.12:8000/audio/{i}?channel=classical");
        panel.ip_station_name = format!("Station {i:02} {}", "A".repeat(80));
        panel.save_ip_station().unwrap();
    }
    panel.ip_station_name = "One too many".into();
    assert!(panel.save_ip_station().is_err());
    render_panel(&mut panel);
    let context = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(PANEL_WIDTH, PANEL_HEIGHT));
    let mut bounds = None;
    for _ in 0..3 {
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
    assert!(screen.contains_rect(bounds.unwrap()), "{bounds:?}");
    assert!(!panel.is_active());
}

#[test]
fn ip_menu_keyboard_navigation_keeps_selection_until_enter_and_scrolls_long_names() {
    let context = egui::Context::default();
    let mut panel = RadioPanel::new();
    for index in 0..24 {
        panel.direct_selected = true;
        panel.direct_url = format!("https://example.com/{index}");
        panel.ip_station_name = format!("{index} {}", "Long name ".repeat(8));
        panel.save_ip_station().unwrap();
    }
    panel.select_ip_station(0);
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(480.0, 320.0));
    let mut menu_id = egui::Id::NULL;
    let mut frame = |panel: &mut RadioPanel, key: Option<egui::Key>, open: bool| {
        let events = key
            .map(|key| {
                vec![egui::Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }]
            })
            .unwrap_or_default();
        let _ = context.run(
            egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    theme::apply(ui);
                    ui.set_max_width(280.0);
                    menu_id = ui.make_persistent_id("rust_radio_station");
                    if open {
                        ui.memory_mut(|memory| memory.open_popup(menu_id.with("popup")));
                    }
                    panel.ip_station_menu(ui);
                });
            },
        );
    };
    frame(&mut panel, None, true);
    frame(&mut panel, Some(egui::Key::End), false);
    assert_eq!(panel.ip_station_index(), 0);
    frame(&mut panel, Some(egui::Key::ArrowUp), false);
    assert_eq!(panel.ip_station_index(), 0);
    frame(&mut panel, Some(egui::Key::Enter), false);
    assert_eq!(panel.saved_station_index, Some(23));
    assert!(!egui::ComboBox::is_open(&context, menu_id));
    assert!(panel.selection_check_pending.is_some());
    panel.stop();
    assert!(!panel.is_active());
}

#[test]
fn switching_input_types_preserves_the_last_saved_ip_station_without_connecting() {
    let mut panel = RadioPanel::new();
    panel.direct_selected = true;
    panel.ip_station_name = "My LAN station".into();
    panel.direct_url = "http://192.168.1.12:8000/live".into();
    panel.save_ip_station().unwrap();
    panel.select_segment(Segment::Network, false);
    panel.select_segment(Segment::Ip, false);
    assert!(panel.direct_selected);
    assert_eq!(panel.ip_station_name(), "My LAN station");
    assert_eq!(panel.direct_url, "http://192.168.1.12:8000/live");
    assert!(!panel.is_active());
    panel.apply_inputs(InputReport::default());
    assert_eq!(panel.segment, Segment::Ip);
    assert_eq!(panel.ip_station_name(), "My LAN station");
}

#[test]
fn listener_status_uses_playback_state_instead_of_cached_verification() {
    let mut panel = RadioPanel::new();
    let mut report = StationValidation::unverified("skylab");
    report.status = ValidationStatus::Unavailable;
    panel.validations.insert("skylab", report);
    for (state, label, working, failed) in [
        (PlaybackState::Stopped, "Ready to listen", false, false),
        (PlaybackState::Connecting, "Connecting", true, false),
        (PlaybackState::Buffering, "Buffering", true, false),
        (PlaybackState::Playing, "Playing", false, false),
        (PlaybackState::Recovering, "Reconnecting", true, false),
        (PlaybackState::Failed, "Unable to play", false, true),
    ] {
        panel.last_session = Some(RadioSession {
            state,
            ..Default::default()
        });
        let status = panel.listening_status();
        assert_eq!(status.label, label);
        assert_eq!(status.working, working);
        assert_eq!(status.failed, failed);
        assert_eq!(panel.source_detail(), "Chillout · lounge · ambient");
        render_panel(&mut panel);
    }
    panel.restart_pending = true;
    assert_eq!(panel.listening_status().label, "Connecting");
    panel.stopping = true;
    let switching = panel.listening_status();
    assert_eq!(switching.label, "Switching station");
    assert!(switching.stop_action);
    panel.restart_pending = false;
    panel.stopping = false;
}

#[test]
fn mute_restores_the_last_listening_volume_without_starting_audio() {
    let mut panel = RadioPanel::new();
    panel.change_volume(37);
    panel.toggle_mute();
    assert_eq!(panel.volume, 0);
    render_panel(&mut panel);
    panel.toggle_mute();
    assert_eq!(panel.volume, 37);
    panel.toggle_mute();
    panel.change_volume(81);
    panel.toggle_mute();
    panel.toggle_mute();
    assert_eq!(panel.volume, 81);
    assert!(!panel.is_active());
}

#[test]
fn playing_surface_has_no_stale_badge_or_decode_diagnostics_and_errors_fit() {
    use egui::epaint::Shape;
    fn texts(shape: &Shape, result: &mut String) {
        match shape {
            Shape::Text(text) => {
                result.push_str(&text.galley.job.text);
                result.push('\n');
            }
            Shape::Vec(shapes) => {
                for shape in shapes {
                    texts(shape, result);
                }
            }
            _ => {}
        }
    }
    let mut panel = RadioPanel::new();
    panel.note = "PLAYING / DECODED AUDIO REACHED OUTPUT".into();
    panel.last_session = Some(RadioSession {
        state: PlaybackState::Playing,
        ..Default::default()
    });
    let context = egui::Context::default();
    let output = context.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(480.0, 320.0),
            )),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.show(ui));
        },
    );
    let mut content = String::new();
    for shape in output.shapes {
        texts(&shape.shape, &mut content);
    }
    assert!(content.contains("Playing"), "{content}");
    assert!(!content.contains("UNVERIFIED"), "{content}");
    assert!(!content.contains("DECODED AUDIO"), "{content}");
    panel.last_session = Some(RadioSession {
        state: PlaybackState::Failed,
        last_error: Some("The station is unavailable. ".repeat(100)),
        ..Default::default()
    });
    render_panel(&mut panel);
    assert_eq!(panel.listening_status().label, "Unable to play");
}

#[test]
fn stream_errors_offer_listener_actions_with_original_diagnostics_retained() {
    let mut panel = RadioPanel::new();
    for (error, useful) in [
        (
            "Unsupported radio stream format: core (probe): no suitable format reader found",
            "unsupported audio format",
        ),
        (
            "Server did not return an audio stream: text/html",
            "check the stream URL",
        ),
        (
            "Radio server responded HTTP 404 Not Found",
            "no longer available",
        ),
        ("Radio connect/header timeout", "Check your connection"),
        (
            "Native audio output device disconnected",
            "select an output",
        ),
        ("Unsupported native sample format: I32", "select an output"),
        (
            "Unsupported native receiver output: UnsupportedChannels",
            "select an output",
        ),
        (
            "Cannot configure native audio stream: device unavailable",
            "select an output",
        ),
    ] {
        panel.last_session = Some(RadioSession {
            state: PlaybackState::Failed,
            last_error: Some(error.into()),
            ..Default::default()
        });
        assert!(panel.listener_error_detail().contains(useful));
        assert_eq!(panel.playback_error().as_deref(), Some(error));
        render_panel(&mut panel);
    }
}

#[test]
fn setup_preserves_six_hour_feed_freshness_without_blocking_playback() {
    let mut panel = RadioPanel::new();
    let mut report = StationValidation::unverified("skylab");
    report.status = ValidationStatus::Decoded;
    report.stage = rust_radio_core::validation::ValidationStage::DecodedAudio;
    report.decoded_frames = 256;
    report.checked_at = Some(SystemTime::now());
    panel.validations.insert("skylab", report);
    assert!(panel.stream_check_summary().contains("passed"));
    panel.validations.get_mut("skylab").unwrap().checked_at = Some(
        SystemTime::now()
            - rust_radio_core::validation::FEED_FRESHNESS_TTL
            - Duration::from_secs(1),
    );
    assert!(panel.stream_check_summary().contains("over 6 hours old"));
    assert_eq!(panel.validation_label("skylab"), "STALE / RECHECK");
    assert!(!panel.is_active());
}
