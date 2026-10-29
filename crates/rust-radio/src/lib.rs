//! Standalone radio module consumed by hosts through a single Rust dependency.
//! Importing it never opens network, audio or SDR resources. The default IP
//! engine is headless; opt into `gui` only when the host uses an egui panel.

pub use rust_radio_core::{
    controller, directory, feed_history, pcm, recovery, stream, validation as validation_types,
    RadioController as SessionController, RadioError, ReceiveBackend, SessionState, Snapshot,
    Source,
};

#[cfg(feature = "ip-radio")]
pub use rust_radio_audio::playback::{
    IpBufferingPreset, IpBufferingProfile, NativePlaybackDiagnostics, NativeRadioEngine,
};
#[cfg(feature = "ip-radio")]
pub use rust_radio_audio::{discovery, output, playback, validation};
#[cfg(feature = "ip-radio")]
pub use rust_radio_ip::{net, probe};

#[cfg(feature = "sdr")]
pub use rust_radio_core::sdr;
#[cfg(feature = "sdr")]
pub use rust_radio_sdr as receiver;

#[cfg(feature = "gui")]
pub use rust_radio_gui::RadioPanel;
