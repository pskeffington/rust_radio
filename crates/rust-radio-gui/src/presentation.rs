//! Listener-facing presentation. Validation history never overrides live playback.
use super::RadioPanel;
use rust_radio_audio::compat_controller::PlaybackState;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ListeningStatus {
    pub label: &'static str,
    pub hint: &'static str,
    pub working: bool,
    pub failed: bool,
    pub stop_action: bool,
}
impl RadioPanel {
    pub(super) fn listening_status(&self) -> ListeningStatus {
        let state = self
            .engine
            .as_ref()
            .map(|engine| engine.status().state)
            .or_else(|| self.last_session.as_ref().map(|session| session.state))
            .unwrap_or(PlaybackState::Stopped);
        let stop_action =
            (self.engine.is_some() && state != PlaybackState::Failed) || self.restart_pending;
        let (label, hint, working, failed) = if self.restart_pending {
            (
                if self.stopping {
                    "Switching station"
                } else {
                    "Connecting"
                },
                "Your station will start shortly.",
                true,
                false,
            )
        } else if self.stopping {
            ("Stopping", "Releasing the current connection.", true, false)
        } else if let Some(check) = &self.validation {
            (
                if check.cancellation_requested() {
                    "Canceling check"
                } else {
                    "Checking station"
                },
                "Press Play to listen, or choose another station.",
                true,
                false,
            )
        } else {
            match state {
                PlaybackState::Playing => ("Playing", "", false, false),
                PlaybackState::Connecting => ("Connecting", "Contacting the station…", true, false),
                PlaybackState::Buffering => ("Buffering", "Getting audio ready…", true, false),
                PlaybackState::Recovering => (
                    "Reconnecting",
                    "The connection dropped. Trying again…",
                    true,
                    false,
                ),
                PlaybackState::Failed => (
                    "Unable to play",
                    "Retry, or choose another station.",
                    false,
                    true,
                ),
                PlaybackState::Stopped => (
                    "Ready to listen",
                    "Choose a station and press Play.",
                    false,
                    false,
                ),
            }
        };
        ListeningStatus {
            label,
            hint,
            working,
            failed,
            stop_action,
        }
    }
    pub(super) fn playback_error(&self) -> Option<String> {
        self.engine
            .as_ref()
            .and_then(|engine| engine.status().last_error)
            .or_else(|| {
                self.last_session
                    .as_ref()
                    .and_then(|session| session.last_error.clone())
            })
    }
    pub(super) fn listener_error_detail(&self) -> String {
        let raw = self
            .playback_error()
            .unwrap_or_else(|| "The station could not be opened. Retry or check setup.".into());
        if self.segment != super::Segment::Ip {
            return raw;
        }
        let message = raw.to_lowercase();
        let friendly = if message.contains("hls") {
            "This station uses HLS streaming, which is not supported yet. Try an MP3 or AAC stream."
        } else if message.contains("unsupported radio")
            || message.contains("no suitable format reader")
        {
            "This stream uses an unsupported audio format. Try another station."
        } else if message.contains("did not return an audio stream") {
            "This address did not return audio. Open IP Radio and check the stream URL."
        } else if message.contains("http 404") || message.contains("http 410") {
            "This stream address is no longer available. Choose another station or update its URL."
        } else if message.contains("http 401") || message.contains("http 403") {
            "The station refused access. Open IP Radio and check the stream URL."
        } else if message.contains("timeout") || message.contains("timed out") {
            "The station took too long to respond. Check your connection and retry."
        } else if message.contains("stream connection failed") {
            "Could not reach the station. Check your connection or stream address, then retry."
        } else if message.contains("audio output")
            || message.contains("audio device")
            || message.contains("native sample format")
            || message.contains("native receiver output")
            || message.contains("cannot configure native audio stream")
        {
            "Your audio output is unavailable. Open IP Radio to select an output, then retry."
        } else {
            return raw;
        };
        friendly.into()
    }

    pub(super) fn output_name(&self) -> &str {
        if self.output == "system" {
            "System default"
        } else {
            self.outputs
                .iter()
                .find(|output| output.id == self.output)
                .map(|output| output.name.as_str())
                .unwrap_or("Selected audio output")
        }
    }
    pub(super) fn change_volume(&mut self, value: u8) {
        self.volume = value;
        if value > 0 {
            self.unmuted_volume = value;
        }
        if let Some(engine) = &self.engine {
            if let Err(error) = engine.set_volume(value) {
                self.note = error;
            }
        }
    }
    pub(super) fn toggle_mute(&mut self) {
        self.change_volume(if self.volume == 0 {
            self.unmuted_volume.max(1)
        } else {
            0
        });
    }
}
