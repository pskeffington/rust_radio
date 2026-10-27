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

#[cfg(feature = "ip-radio")]
#[test]
fn headless_engine_validates_commands_before_opening_resources() {
    assert!(rust_radio::NativeRadioEngine::start_internet("unknown", 50, "system").is_err());
    assert!(rust_radio::NativeRadioEngine::start_internet("skylab", 101, "system").is_err());
}

#[cfg(feature = "sdr")]
#[test]
fn receive_only_tuner_contract_is_independent_of_native_audio() {
    use rust_radio::sdr::{Demodulation, RadioTuner, SimulatedRadio, Tuning};
    let mut tuner = RadioTuner::new(SimulatedRadio::default()).unwrap();
    tuner
        .tune(Tuning {
            frequency_hz: 100_100_000,
            sample_rate_hz: 1_024_000,
            demodulation: Demodulation::Wfm,
            gain_db_tenths: None,
        })
        .unwrap();
    let mut samples = [1; 32];
    assert_eq!(tuner.read_iq(&mut samples).unwrap(), samples.len());
    assert_eq!(samples, [0; 32]);
    assert!(tuner.capabilities().name.contains("SIMULATED"));
}
