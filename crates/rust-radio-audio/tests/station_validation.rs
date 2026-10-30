use rust_radio_audio::validation::{validate_station, validate_with};
use rust_radio_core::stream::{ApprovedEndpoint, HttpResponse, RadioHttp, StreamError};
use rust_radio_core::validation::{StationValidation, ValidationStage, ValidationStatus};
use std::cell::RefCell;
use std::io::Cursor;

const ROOT: &str = "https://stream.radioparadise.com/mp3-128";
const AUDIO: &str = ROOT;
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
        assert_eq!(target.url(), ROOT);
        let body = if matches!(
            self.mime,
            "audio/x-mpegurl" | "application/x-mpegurl" | "application/vnd.apple.mpegurl"
        ) {
            self.playlist.clone()
        } else {
            self.body.clone()
        };
        Ok(HttpResponse {
            status: self.status,
            content_type: Some(self.mime.into()),
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
        let report = validate_with(&fixture, "radio-paradise").unwrap();
        assert!(report.is_decoded(), "{report:?}");
        assert_eq!(report.status, ValidationStatus::Decoded);
        assert_eq!(report.stage, ValidationStage::DecodedAudio);
        assert_eq!(report.resolved_url.as_deref(), Some(AUDIO));
        assert_eq!(report.codec.as_deref(), Some(codec));
        assert_eq!(report.sample_rate, Some(44_100));
        assert_eq!(report.channels, Some(1));
        assert!(report.checked_at.is_some());
        assert_eq!(fixture.requests.borrow().as_slice(), [ROOT]);
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
    let report = validate_with(&Fixture::audio("audio/aac", &bytes), "radio-paradise").unwrap();
    assert!(report.is_decoded(), "{report:?}");
}

#[test]
fn station_validation_rejects_publishers_without_third_party_player_support() {
    let mut fixture = Fixture::audio("audio/x-mpegurl", MP3);
    fixture.playlist = b"#EXTM3U\nhttps://somafm.com/m3u/groovesalad.m3u\n".into();
    let report = validate_with(&fixture, "radio-paradise").unwrap();
    assert_eq!(report.status, ValidationStatus::InvalidStream);
    assert_eq!(fixture.requests.borrow().as_slice(), [ROOT]);
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
        let report = validate_with(&fixture, "radio-paradise").unwrap();
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
    let report = validate_with(&Fixture::audio("audio/wav", &wav), "radio-paradise").unwrap();
    assert_eq!(report.status, ValidationStatus::Unsupported, "{report:?}");
    assert_eq!(report.stage, ValidationStage::StreamResponse);
    assert!(!report.is_decoded());

    let mut unavailable = Fixture::audio("audio/mpeg", MP3);
    unavailable.status = 503;
    let report = validate_with(&unavailable, "radio-paradise").unwrap();
    assert_eq!(report.status, ValidationStatus::Unavailable);
    assert_eq!(report.stage, ValidationStage::EndpointReached);
    assert_eq!(unavailable.requests.borrow().len(), 1);

    let mut missing = Fixture::audio("audio/mpeg", MP3);
    missing.status = 404;
    let report = validate_with(&missing, "radio-paradise").unwrap();
    assert_eq!(report.status, ValidationStatus::InvalidStream);
    assert_eq!(report.stage, ValidationStage::EndpointReached);
    assert!(report.detail.contains("404"));

    let mut rate_limited = Fixture::audio("audio/mpeg", MP3);
    rate_limited.status = 429;
    let report = validate_with(&rate_limited, "radio-paradise").unwrap();
    assert_eq!(report.status, ValidationStatus::Unavailable);
}

#[test]
fn unsafe_and_hls_playlists_fail_before_requesting_their_entries() {
    for (playlist, status) in [
        (
            "#EXTM3U\nhttps://127.0.0.1/private",
            ValidationStatus::InvalidStream,
        ),
        (
            "#EXTM3U\n#EXT-X-VERSION:3\nhttps://somafm.com/segment.ts",
            ValidationStatus::Unsupported,
        ),
    ] {
        let mime = if status == ValidationStatus::Unsupported {
            "application/vnd.apple.mpegurl"
        } else {
            "audio/x-mpegurl"
        };
        let mut fixture = Fixture::audio(mime, MP3);
        fixture.playlist = playlist.as_bytes().into();
        let report = validate_with(&fixture, "radio-paradise").unwrap();
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

struct ConnectionFailure;

impl RadioHttp for ConnectionFailure {
    type Body = Cursor<Vec<u8>>;

    fn get(&self, _: &ApprovedEndpoint) -> Result<HttpResponse<Self::Body>, StreamError> {
        Err(StreamError::TransportConnectionFailed)
    }
}

#[test]
fn connection_failure_is_reported_safely_without_changing_unavailable_status() {
    let report = validate_with(&ConnectionFailure, "skylab").unwrap();
    assert_eq!(report.status, ValidationStatus::Unavailable);
    assert_eq!(report.stage, ValidationStage::NotChecked);
    assert_eq!(
        report.detail,
        "Radio stream connection failed before a response"
    );
    assert!(!report.is_decoded());
}

#[test]
fn unknown_stations_fail_before_transport_or_audio_initialization() {
    let fixture = Fixture::audio("audio/mpeg", MP3);
    assert!(validate_with(&fixture, "unknown").is_err());
    assert!(fixture.requests.borrow().is_empty());
    assert!(validate_station("https://localhost/admin").is_err());
}
