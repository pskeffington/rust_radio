//! Receive-only SDR integration. No hardware is opened on import or discovery
//! of pure module capabilities. Physical discovery/open/capture are explicit.
#![forbid(unsafe_code)]

mod capture;
pub use capture::{capture, CaptureReport, MAX_CAPTURE_DURATION, MAX_IQ_VALUES};

#[cfg(feature = "soapy")]
mod soapy;
#[cfg(feature = "soapy")]
pub use soapy::{discover, ReceiverInfo, SampleRateRange, SoapyReceiver};

/// SDK-independent SDRconnect WebSocket API, including nRSP-ST bridges.
#[cfg(feature = "sdrconnect")]
pub mod sdrconnect;
