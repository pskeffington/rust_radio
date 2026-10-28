//! Receive-only SDR integration. No hardware is opened on import or discovery
//! of pure module capabilities. Physical discovery/open/capture are explicit.
#![forbid(unsafe_code)]

mod capture;
pub use capture::{capture, CaptureReport, MAX_CAPTURE_DURATION, MAX_IQ_VALUES};
mod wfm;
pub use wfm::{stream_mono_wfm, WfmReport};

#[cfg(feature = "soapy")]
mod soapy;
#[cfg(feature = "soapy")]
pub use soapy::{discover, ReceiverInfo, SampleRateRange, SoapyReceiver};

/// SDK-independent SDRconnect WebSocket API, including nRSP-ST bridges.
#[cfg(feature = "sdrconnect")]
pub mod sdrconnect;

/// Direct, read-only reachability checks for the nRSP-ST network services.
pub mod nrsp;

/// Direct nRSP-ST session control and compressed audio, without SDRconnect.
#[cfg(feature = "nrsp-native")]
pub mod nrsp_native;
