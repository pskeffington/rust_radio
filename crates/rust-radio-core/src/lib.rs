//! Portable radio domain types. No network, audio device or RF I/O on construction.
//! Backend implementations belong behind opt-in features.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Internet {
        station_id: String,
    },
    Sdr {
        device_id: String,
        frequency_hz: u64,
    },
    Satellite {
        receiver_id: String,
        frequency_hz: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState {
    Stopped,
    Connecting,
    Buffering,
    Playing,
    Recovering,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RadioError {
    InvalidVolume,
    AlreadyActive,
    NoActiveSession,
    StaleGeneration,
    InvalidTransition,
    UnknownStation,
    Backend(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub generation: u64,
    pub state: SessionState,
    pub source: Option<Source>,
    pub volume: u8,
}

pub trait ReceiveBackend {
    /// Explicit operator action; must not be invoked during discovery or construction.
    fn start(&mut self, source: &Source, generation: u64) -> Result<(), RadioError>;
    /// Must actively cancel worker I/O, not only hide the previous generation.
    fn stop(&mut self, generation: u64) -> Result<(), RadioError>;
}

#[derive(Debug)]
pub struct RadioController {
    snapshot: Snapshot,
}

impl Default for RadioController {
    fn default() -> Self {
        Self {
            snapshot: Snapshot {
                generation: 0,
                state: SessionState::Stopped,
                source: None,
                volume: 50,
            },
        }
    }
}

impl RadioController {
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    pub fn set_volume(&mut self, value: u8) -> Result<(), RadioError> {
        if value > 100 {
            return Err(RadioError::InvalidVolume);
        }
        self.snapshot.volume = value;
        Ok(())
    }

    pub fn request_start(&mut self, source: Source) -> Result<u64, RadioError> {
        if let Source::Internet { ref station_id } = source {
            if directory::station_by_id(station_id).is_none() {
                return Err(RadioError::UnknownStation);
            }
        }
        if self.snapshot.state != SessionState::Stopped && self.snapshot.state != SessionState::Failed
        {
            return Err(RadioError::AlreadyActive);
        }
        self.snapshot.generation = self.snapshot.generation.wrapping_add(1);
        self.snapshot.source = Some(source);
        self.snapshot.state = SessionState::Connecting;
        Ok(self.snapshot.generation)
    }

    /// Validate a curated station before attempting any receiver I/O.
    pub fn request_internet_station(&mut self, station_id: &str) -> Result<u64, RadioError> {
        if directory::station_by_id(station_id).is_none() {
            return Err(RadioError::UnknownStation);
        }
        self.request_start(Source::Internet {
            station_id: station_id.to_owned(),
        })
    }

    pub fn acknowledge_state(
        &mut self,
        generation: u64,
        state: SessionState,
    ) -> Result<(), RadioError> {
        if generation != self.snapshot.generation {
            return Err(RadioError::StaleGeneration);
        }
        let allowed = matches!(
            (self.snapshot.state, state),
            (SessionState::Connecting, SessionState::Buffering)
                | (SessionState::Connecting, SessionState::Recovering)
                | (SessionState::Buffering, SessionState::Playing)
                | (SessionState::Buffering, SessionState::Recovering)
                | (SessionState::Playing, SessionState::Recovering)
                | (SessionState::Recovering, SessionState::Buffering)
                | (SessionState::Recovering, SessionState::Connecting)
        ) || (state == SessionState::Failed && self.snapshot.state != SessionState::Stopped);
        if !allowed {
            return Err(RadioError::InvalidTransition);
        }
        self.snapshot.state = state;
        Ok(())
    }

    /// Invalidates delayed events; the host must additionally stop backend workers.
    pub fn request_stop(&mut self) -> u64 {
        self.snapshot.generation = self.snapshot.generation.wrapping_add(1);
        self.snapshot.state = SessionState::Stopped;
        self.snapshot.source = None;
        self.snapshot.generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_events_cannot_restart_stopped_session() {
        let mut control = RadioController::default();
        let generation = control
            .request_start(Source::Internet {
                station_id: "skylab".into(),
            })
            .unwrap();
        control.request_stop();
        assert_eq!(
            control.acknowledge_state(generation, SessionState::Playing),
            Err(RadioError::StaleGeneration)
        );
        assert_eq!(control.snapshot().state, SessionState::Stopped);
    }
    #[test]
    fn cannot_claim_playing_before_buffering() {
        let mut control = RadioController::default();
        let generation = control
            .request_start(Source::Internet {
                station_id: "skylab".into(),
            })
            .unwrap();
        assert_eq!(
            control.acknowledge_state(generation, SessionState::Playing),
            Err(RadioError::InvalidTransition)
        );
        assert_eq!(control.snapshot().state, SessionState::Connecting);
        control
            .acknowledge_state(generation, SessionState::Buffering)
            .unwrap();
        control
            .acknowledge_state(generation, SessionState::Playing)
            .unwrap();
    }
    #[test]
    fn invalid_volume_is_rejected() {
        let mut control = RadioController::default();
        assert_eq!(control.set_volume(101), Err(RadioError::InvalidVolume));
        assert_eq!(control.snapshot().volume, 50);
    }
    #[test]
    fn unapproved_station_does_not_start_session() {
        let mut control = RadioController::default();
        assert_eq!(
            control.request_internet_station("https://localhost/admin"),
            Err(RadioError::UnknownStation)
        );
        assert_eq!(control.snapshot().generation, 0);
        assert_eq!(control.snapshot().state, SessionState::Stopped);
        assert!(control.request_internet_station("skylab").is_ok());
    }
    #[test]
    fn cannot_start_second_session() {
        let mut control = RadioController::default();
        control
            .request_start(Source::Internet {
                station_id: "skylab".into(),
            })
            .unwrap();
        assert_eq!(
            control.request_start(Source::Internet {
                station_id: "groovesalad".into(),
            }),
            Err(RadioError::AlreadyActive)
        );
    }
}

/// DSP-independent PCM conversion, extracted from the Rust-Boy baseline.
pub mod pcm;
/// Pure retry and output-recovery policy, extracted from the Rust-Boy baseline.
pub mod recovery;

/// Curated broadcaster catalog and source selection, extracted from Rust-Boy.
pub mod directory;
/// Bounded HTTPS stream-response, playlist and ICY parser (network-free).
pub mod stream;
