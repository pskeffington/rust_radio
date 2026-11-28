use rust_radio::{
    controller::{PlaybackState, RadioCommand, RadioController},
    directory, recovery, SessionState,
};

#[test]
fn host_control_contract_is_shared_and_initialization_is_offline() {
    let stations = directory::curated_stations();
    assert_eq!(stations[0].id, "skylab");
    let mut radio = RadioController::default();
    let state: SessionState = radio.state().state;
    assert_eq!(state, PlaybackState::Stopped);
    assert!(!recovery::restart_after_output_change(state, false));
    radio.apply(RadioCommand::SetVolume(35)).unwrap();
    assert_eq!(radio.state().volume_percent, 35);
    assert!(radio.state().station_id.is_none());
}

#[test]
fn headless_host_can_read_shared_feed_history_without_enabling_gui_or_io() {
    let history = rust_radio::feed_history::parse_feed_history(include_str!(
        "../../../testdata/feed-health-v1.json"
    ))
    .unwrap();
    let skylab = history.stations.get("skylab").unwrap();
    assert_eq!(
        skylab.status,
        rust_radio::validation_types::ValidationStatus::Decoded
    );
    assert_eq!(
        skylab.stage,
        rust_radio::validation_types::ValidationStage::DecodedAudio
    );
    assert_eq!(skylab.attempts, 1);
    assert_eq!(
        skylab.health_at(
            std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000)
        ),
        rust_radio::validation_types::FeedHealth::Fresh
    );
}

#[cfg(feature = "ip-radio")]
#[test]
fn headless_engine_validates_commands_before_opening_resources() {
    assert!(rust_radio::NativeRadioEngine::start_internet("unknown", 50, "system").is_err());
    assert!(rust_radio::NativeRadioEngine::start_internet("skylab", 101, "system").is_err());
}
