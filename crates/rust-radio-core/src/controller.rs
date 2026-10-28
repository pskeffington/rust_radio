//! Native, source-independent radio playback control.
//!
//! Pure state machine shared by embedded hosts and native playback workers.
//! No network, audio output, subprocesses or autoplay.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    Internet,
    Sdr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Connecting,
    Buffering,
    Playing,
    Recovering,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadioSession {
    pub state: PlaybackState,
    pub input: Option<InputKind>,
    pub station_id: Option<String>,
    pub volume_percent: u8,
    /// Changes on every start/stop; workers must drop stale results.
    pub generation: u64,
    pub last_error: Option<String>,
}

impl Default for RadioSession {
    fn default() -> Self {
        Self {
            state: PlaybackState::Stopped,
            input: None,
            station_id: None,
            volume_percent: 60,
            generation: 0,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadioCommand {
    PlayInternet { station_id: String },
    PlayStream,
    PlaySdr,
    Stop,
    SetVolume(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackEvent {
    Connected,
    Buffered,
    PlaybackStarted,
    ConnectionLost,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlError {
    UnknownStation,
    InvalidVolume,
    InvalidTransition,
}

#[derive(Default)]
pub struct RadioController {
    session: RadioSession,
}

impl RadioController {
    pub fn state(&self) -> &RadioSession {
        &self.session
    }

    pub fn apply(&mut self, command: RadioCommand) -> Result<u64, ControlError> {
        match command {
            RadioCommand::PlayInternet { station_id } => {
                if crate::directory::station_by_id(&station_id).is_none() {
                    return Err(ControlError::UnknownStation);
                }
                self.session.generation = self.session.generation.wrapping_add(1);
                self.session.state = PlaybackState::Connecting;
                self.session.input = Some(InputKind::Internet);
                self.session.station_id = Some(station_id);
                self.session.last_error = None;
            }
            RadioCommand::PlayStream => {
                self.session.generation = self.session.generation.wrapping_add(1);
                self.session.state = PlaybackState::Connecting;
                self.session.input = Some(InputKind::Internet);
                self.session.station_id = None;
                self.session.last_error = None;
            }
            RadioCommand::PlaySdr => {
                self.session.generation = self.session.generation.wrapping_add(1);
                self.session.state = PlaybackState::Connecting;
                self.session.input = Some(InputKind::Sdr);
                self.session.station_id = None;
                self.session.last_error = None;
            }
            RadioCommand::Stop => {
                self.session.generation = self.session.generation.wrapping_add(1);
                self.session.state = PlaybackState::Stopped;
                self.session.input = None;
                self.session.station_id = None;
                self.session.last_error = None;
            }
            RadioCommand::SetVolume(value) => {
                if value > 100 {
                    return Err(ControlError::InvalidVolume);
                }
                self.session.volume_percent = value;
            }
        }
        Ok(self.session.generation)
    }

    /// Apply only events produced by the currently active worker generation.
    /// Returns false for stale events or disallowed state changes.
    pub fn event(&mut self, generation: u64, event: PlaybackEvent) -> bool {
        if generation != self.session.generation || self.session.state == PlaybackState::Stopped {
            return false;
        }
        let next = match (&self.session.state, &event) {
            (PlaybackState::Connecting | PlaybackState::Recovering, PlaybackEvent::Connected) => {
                PlaybackState::Buffering
            }
            (PlaybackState::Buffering, PlaybackEvent::Buffered) => PlaybackState::Buffering,
            (PlaybackState::Buffering, PlaybackEvent::PlaybackStarted) => PlaybackState::Playing,
            (
                PlaybackState::Playing
                | PlaybackState::Buffering
                | PlaybackState::Connecting
                | PlaybackState::Recovering,
                PlaybackEvent::ConnectionLost,
            ) => PlaybackState::Recovering,
            (_, PlaybackEvent::Error(_)) => PlaybackState::Failed,
            _ => return false,
        };
        if let PlaybackEvent::Error(message) = event {
            self.session.last_error = Some(message);
        }
        self.session.state = next;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_station_starts_only_connecting_not_playing() {
        let mut radio = RadioController::default();
        radio
            .apply(RadioCommand::PlayInternet {
                station_id: "skylab".into(),
            })
            .unwrap();
        assert_eq!(radio.state().state, PlaybackState::Connecting);
        assert_eq!(radio.state().input, Some(InputKind::Internet));
    }

    #[test]
    fn no_untrusted_station_or_invalid_gain() {
        let mut radio = RadioController::default();
        assert_eq!(
            radio.apply(RadioCommand::PlayInternet {
                station_id: "bad".into()
            }),
            Err(ControlError::UnknownStation)
        );
        assert_eq!(
            radio.apply(RadioCommand::SetVolume(101)),
            Err(ControlError::InvalidVolume)
        );
        assert_eq!(radio.state().generation, 0);
    }

    #[test]
    fn stale_worker_cannot_restart_after_stop() {
        let mut radio = RadioController::default();
        let old = radio
            .apply(RadioCommand::PlayInternet {
                station_id: "skylab".into(),
            })
            .unwrap();
        assert!(radio.event(old, PlaybackEvent::Connected));
        radio.apply(RadioCommand::Stop).unwrap();
        assert!(!radio.event(old, PlaybackEvent::PlaybackStarted));
        assert_eq!(radio.state().state, PlaybackState::Stopped);
    }

    #[test]
    fn disconnected_stream_reenters_buffering_on_same_generation() {
        let mut radio = RadioController::default();
        let generation = radio
            .apply(RadioCommand::PlayInternet {
                station_id: "skylab".into(),
            })
            .unwrap();
        assert!(radio.event(generation, PlaybackEvent::ConnectionLost));
        assert_eq!(radio.state().state, PlaybackState::Recovering);
        assert!(radio.event(generation, PlaybackEvent::Connected));
        assert!(radio.event(generation, PlaybackEvent::Buffered));
        assert!(radio.event(generation, PlaybackEvent::PlaybackStarted));
        assert_eq!(radio.state().state, PlaybackState::Playing);
        radio.apply(RadioCommand::Stop).unwrap();
        assert!(!radio.event(generation, PlaybackEvent::Connected));
    }

    #[test]
    fn playback_needs_actual_worker_events() {
        let mut radio = RadioController::default();
        let gen = radio
            .apply(RadioCommand::PlayInternet {
                station_id: "radio-paradise".into(),
            })
            .unwrap();
        assert!(!radio.event(gen, PlaybackEvent::PlaybackStarted));
        assert!(radio.event(gen, PlaybackEvent::Connected));
        assert!(radio.event(gen, PlaybackEvent::Buffered));
        assert!(radio.event(gen, PlaybackEvent::PlaybackStarted));
        assert_eq!(radio.state().state, PlaybackState::Playing);
    }
}
