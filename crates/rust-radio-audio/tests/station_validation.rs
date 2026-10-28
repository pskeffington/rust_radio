use rust_radio_audio::validation::{validate_station, validate_with};
use rust_radio_core::stream::{ApprovedEndpoint, HttpResponse, RadioHttp, StreamError};
use rust_radio_core::validation::{StationValidation, ValidationStage, ValidationStatus};
use std::cell::RefCell;
use std::io::Cursor;

const ROOT: &str = "https://somafm.com/m3u/groovesalad.m3u";
const AUDIO: &str = "https://ice5.somafm.com/groovesalad-128-mp3";
const MP3: &[u8] = include_bytes!("fixtures/tone.mp3");
const AAC: &[u8] = include_bytes!("fixtures/tone.aac");

struct Fixture {
    mime: &'static str,
    body: Vec<u8>,
    playlist: Vec<u8>,
    status: u16,
    requests: RefCell<Vec<String>>,
}

impl Fixture {
    fn audio(mime: &'static str, body: &[u8]) -> Self {
        Self {
            mime,
            body: body.into(),
            playlist: format!("#EXTM3U\n{AUDIO}\n").into_bytes(),
            status: 200,
            requests: RefCell::new(Vec::new()),
        }
    }
}

impl RadioHttp for Fixture {
    type Body = Cursor<Vec<u8>>;

    fn get(&self, target: &ApprovedEndpoint) -> Result<HttpResponse<Self::Body>, StreamError> {
        self.requests.borrow_mut().push(target.url().to_owned());
        let (mime, body) = match target.url() {
            ROOT => ("audio/x-mpegurl", self.playlist.clone()),
            AUDIO => (self.mime, self.body.clone()),
            _ => panic!("Validation requested an unexpected endpoint"),
        };
        Ok(HttpResponse {
            status: self.status,
            content_type: Some(mime.into()),
            redirect_location: None,
            icy_metaint: None,
            body: Cursor::new(body),
        })
    }
}

#[test]
fn synthetic_mp3_and_aac_decode_without_audio_hardware() {
    for (mime, bytes, codec) in [("audio/mpeg", MP3, "mp3"), ("audio/aac", AAC, "aac")] {
        let fixture = Fixture::audio(mime, bytes);
        let report = validate_with(&fixture, "groovesalad").unwrap();
        assert!(report.is_decoded(), "{report:?}");
        assert_eq!(report.status, ValidationStatus::Decoded);
        assert_eq!(report.stage, ValidationStage::DecodedAudio);
        assert_eq!(report.resolved_url.as_deref(), Some(AUDIO));
        assert_eq!(report.codec.as_deref(), Some(codec));
        assert_eq!(report.sample_rate, Some(44_100));
        assert_eq!(report.channels, Some(1));
        assert!(report.checked_at.is_some());
        assert_eq!(fixture.requests.borrow().as_slice(), [ROOT, AUDIO]);
    }
}

#[test]
fn mpeg2_adts_aac_decodes_without_changing_frame_payload() {
    let mut bytes = AAC.to_vec();
    let mut pos = 0;
    while pos + 7 <= bytes.len() {
        assert_eq!(&bytes[pos..pos + 2], &[0xff, 0xf1]);
        let length = ((bytes[pos + 3] as usize & 3) << 11)
            | ((bytes[pos + 4] as usize) << 3)
            | ((bytes[pos + 5] as usize) >> 5);
        assert!(length >= 7 && pos + length <= bytes.len());
        bytes[pos + 1] = 0xf9;
        pos += length;
    }
    assert_eq!(pos, bytes.len());
    let report = validate_with(&Fixture::audio("audio/aac", &bytes), "groovesalad").unwrap();
    assert!(report.is_decoded(), "{report:?}");
}

#[test]
fn legacy_http_playlist_uses_only_https_requests_and_decodes_pcm() {
    let mut fixture = Fixture::audio("audio/mpeg", MP3);
    fixture.playlist = b"#EXTM3U\nhttp://ice5.somafm.com/groovesalad-128-mp3\n".into();
    let report = validate_with(&fixture, "groovesalad").unwrap();
    assert!(report.is_decoded(), "{report:?}");
    assert_eq!(fixture.requests.borrow().as_slice(), [ROOT, AUDIO]);
    assert_eq!(report.resolved_url.as_deref(), Some(AUDIO));
}

#[test]
fn successful_http_and_claimed_audio_type_are_not_decode_evidence() {
    for (mime, body, status) in [
        (
            "text/html",
            b"<html>not radio</html>".as_slice(),
            ValidationStatus::InvalidStream,
        ),
        (
            "audio/mpeg",
            b"".as_slice(),
            ValidationStatus::InvalidStream,
        ),
        ("audio/mpeg", &MP3[..10], ValidationStatus::InvalidStream),
    ] {
        let fixture = Fixture::audio(mime, body);
        let report = validate_with(&fixture, "groovesalad").unwrap();
        assert!(!report.is_decoded(), "{report:?}");
        assert_eq!(report.status, status, "{report:?}");
        assert_eq!(report.decoded_frames, 0);
        assert!(report.checked_at.is_some());
    }
}

#[test]
fn unsupported_format_is_distinct_from_an_unavailable_station() {
    // A PCM WAV header: WAV support is deliberately disabled in this radio decoder.
    let mut wav = b"RIFF\x24\x10\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x01\x00\x44\xac\x00\x00\x88\x58\x01\x00\x02\x00\x10\x00data\x00\x10\x00\x00".to_vec();
    wav.extend_from_slice(&[0; 4096]);
    let report = validate_with(&Fixture::audio("audio/wav", &wav), "groovesalad").unwrap();
    assert_eq!(report.status, ValidationStatus::Unsupported, "{report:?}");
    assert_eq!(report.stage, ValidationStage::StreamResponse);
    assert!(!report.is_decoded());

    let mut unavailable = Fixture::audio("audio/mpeg", MP3);
    unavailable.status = 503;
    let report = validate_with(&unavailable, "groovesalad").unwrap();
    assert_eq!(report.status, ValidationStatus::Unavailable);
    assert_eq!(report.stage, ValidationStage::EndpointReached);
    assert_eq!(unavailable.requests.borrow().len(), 1);
}

#[test]
fn unsafe_and_hls_playlists_fail_before_requesting_their_entries() {
    for (playlist, status) in [
        (
            "#EXTM3U\nhttps://127.0.0.1/private",
            ValidationStatus::InvalidStream,
        ),
        (
            "#EXTM3U\n#EXT-X-VERSION:3\nhttps://ice5.somafm.com/segment.ts",
            ValidationStatus::Unsupported,
        ),
    ] {
        let mut fixture = Fixture::audio("audio/mpeg", MP3);
        fixture.playlist = playlist.as_bytes().into();
        let report = validate_with(&fixture, "groovesalad").unwrap();
        assert_eq!(report.status, status);
        assert_eq!(fixture.requests.borrow().as_slice(), [ROOT]);
        assert!(!report.is_decoded());
    }
}

struct Offline;

impl RadioHttp for Offline {
    type Body = Cursor<Vec<u8>>;

    fn get(&self, _: &ApprovedEndpoint) -> Result<HttpResponse<Self::Body>, StreamError> {
        Err(StreamError::TransportUnavailable)
    }
}

#[test]
fn network_failure_and_unverified_catalog_entries_never_claim_audio() {
    let unverified = StationValidation::unverified("skylab");
    assert_eq!(unverified.status, ValidationStatus::Unverified);
    assert_eq!(unverified.stage, ValidationStage::NotChecked);
    assert!(unverified.checked_at.is_none());
    assert!(!unverified.is_decoded());
    let report = validate_with(&Offline, "skylab").unwrap();
    assert_eq!(report.status, ValidationStatus::Unavailable);
    assert_eq!(report.stage, ValidationStage::NotChecked);
    assert!(report.checked_at.is_some());
    assert!(!report.is_decoded());
}

#[test]
fn unknown_stations_fail_before_transport_or_audio_initialization() {
    let fixture = Fixture::audio("audio/mpeg", MP3);
    assert!(validate_with(&fixture, "unknown").is_err());
    assert!(fixture.requests.borrow().is_empty());
    assert!(validate_station("https://localhost/admin").is_err());
}
