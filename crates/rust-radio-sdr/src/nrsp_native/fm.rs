//! Connection-local FM broadcast audio settings from the native browser contract.
use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum Deemphasis {
    Off = 0,
    Us50 = 50,
    #[default]
    Us75 = 75,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FmSettings {
    pub stereo: bool,
    pub deemphasis: Deemphasis,
    pub filter_enabled: bool,
    pub low_cut_hz: u32,
    pub high_cut_hz: u32,
    pub limiter: bool,
}
impl Default for FmSettings {
    fn default() -> Self {
        Self {
            stereo: true,
            deemphasis: Deemphasis::Us75,
            filter_enabled: true,
            low_cut_hz: 30,
            high_cut_hz: 15_000,
            limiter: true,
        }
    }
}
impl FmSettings {
    pub fn validate(self) -> Result<(), Error> {
        if self.low_cut_hz > 1_000
            || !(3_000..=15_000).contains(&self.high_cut_hz)
            || self.low_cut_hz >= self.high_cut_hz
        {
            return Err(Error::InvalidFmSettings);
        }
        Ok(())
    }
    fn properties(self) -> [(u32, usize, u32); 6] {
        [
            (1, 2, u32::from(self.filter_enabled)),
            (2, 3, u32::from(self.limiter)),
            (5, 6, u32::from(self.stereo)),
            (6, 7, self.deemphasis as u32),
            (7, 8, self.low_cut_hz),
            (8, 9, self.high_cut_hz),
        ]
    }
}
impl Client {
    /// Apply only receiver 1's FM audio properties. Other receivers and physical
    /// antenna/gain settings are unchanged. Successful writes are not readback.
    pub fn set_fm(&mut self, settings: FmSettings, cancel: &AtomicBool) -> Result<(), Error> {
        settings.validate()?;
        if !self.receiver_owned || self.status.configured_mode != Some(Mode::Wfm) {
            return Err(Error::Protocol(
                "FM settings require an active WFM receiver",
            ));
        }
        if settings.stereo && !self.status.stereo_allowed {
            return Err(Error::StereoUnavailable);
        }
        let deadline = Instant::now() + DEADLINE;
        for (property, index, value) in settings.properties() {
            let mut words = [0; 12];
            words[0] = 1;
            words[1] = property;
            words[index] = value;
            self.send(40, &words, deadline, cancel)?;
        }
        if self
            .status
            .fm_settings
            .map_or(true, |previous| previous.stereo != settings.stereo)
        {
            self.status.stereo_locked = None;
        }
        self.status.fm_settings = Some(settings);
        Ok(())
    }
    pub(super) fn stereo_status(&mut self, payload: &[u8]) -> Result<(), Error> {
        if payload.len() != 8 || word(payload, 1)? > 1 {
            return Err(Error::Protocol("Invalid FM stereo status"));
        }
        if word(payload, 0)? == 1 {
            self.status.stereo_locked = Some(word(payload, 1)? == 1);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{command, fixture, server_frame};
    use super::*;
    #[test]
    fn fm_properties_use_exact_slots_and_report_only_primary_stereo_lock() {
        let settings = FmSettings {
            deemphasis: Deemphasis::Us50,
            low_cut_hz: 150,
            high_cut_hz: 8000,
            ..Default::default()
        };
        let (endpoint, server) = fixture(false, move |mut socket, _| {
            for kind in [28, 2, 32, 34, 33, 39, 30, 59] {
                assert_eq!(command(&mut socket).0, kind);
            }
            for (property, index, value) in [
                (1, 2, 1),
                (2, 3, 1),
                (5, 6, 1),
                (6, 7, 50),
                (7, 8, 150),
                (8, 9, 8000),
            ] {
                let (kind, words) = command(&mut socket);
                assert_eq!(kind, 40);
                assert_eq!(words.len(), 12);
                let mut expected = vec![0; 12];
                expected[0] = 1;
                expected[1] = property;
                expected[index] = value;
                assert_eq!(words, expected);
            }
            assert_eq!(command(&mut socket).0, 17);
            let session = Session {
                mask: 0x12345678,
                key: [11, 22, 33, 44],
            };
            for payload in [[0, 0, 0, 2, 0, 0, 0, 1], [0, 0, 0, 1, 0, 0, 0, 1]] {
                socket
                    .send(Message::Binary(
                        server_frame(214, &payload, &session).into(),
                    ))
                    .unwrap();
            }
            socket
                .send(Message::Binary(
                    server_frame(211, &[0, 0, 0, 1, 0xf8, 0xff, 0xfe], &session).into(),
                ))
                .unwrap();
            assert_eq!(command(&mut socket), (30, vec![1, 0]));
        });
        let cancel = AtomicBool::new(false);
        let mut client = Client::connect(endpoint, &cancel).unwrap();
        client
            .start(
                Config::new(99_100_000, Mode::Wfm, 200_000).unwrap(),
                &cancel,
            )
            .unwrap();
        client.set_fm(settings, &cancel).unwrap();
        assert_eq!(client.status().fm_settings, Some(settings));
        assert_eq!(client.status().stereo_locked, None);
        client.stream_audio(&cancel, |_| false).unwrap();
        assert_eq!(client.status().stereo_locked, Some(true));
        client.stereo_status(&[0, 0, 0, 2, 0, 0, 0, 0]).unwrap();
        assert_eq!(client.status().stereo_locked, Some(true));
        assert!(client.stereo_status(&[0, 0, 0, 1, 0, 0, 0, 2]).is_err());
        client.stop().unwrap();
        drop(client);
        server.join().unwrap();
    }
    #[test]
    fn fm_filter_bounds_are_validated_before_io() {
        for settings in [
            FmSettings {
                low_cut_hz: 1001,
                ..Default::default()
            },
            FmSettings {
                high_cut_hz: 15001,
                ..Default::default()
            },
            FmSettings {
                high_cut_hz: 0,
                ..Default::default()
            },
        ] {
            assert_eq!(settings.validate(), Err(Error::InvalidFmSettings));
        }
        assert_eq!(Deemphasis::Us50 as u32, 50);
        assert_eq!(Deemphasis::Us75 as u32, 75);
    }
}
