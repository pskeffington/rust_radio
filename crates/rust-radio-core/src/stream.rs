//! Native IP-radio response parsing and safe, bounded stream preparation.
//!
//! This module deliberately makes NO network requests. A future in-process
//! HTTPS transport implements RadioHttp; fixture transports exercise the
//! entire playlist/redirect/ICY path without audio devices or subprocesses.

use crate::directory::station_by_id;
use std::fmt;
use std::io::{self, Read};

pub const MAX_PLAYLIST_BYTES: usize = 16 * 1024;
pub const MAX_PLAYLIST_ENTRIES: usize = 16;
pub const MAX_REQUEST_HOPS: usize = 5;
pub const MAX_AUDIO_READ: usize = 8192;
pub const MAX_ICY_INTERVAL: usize = 16 * 1024 * 1024;
const MAX_TITLE_BYTES: usize = 256;

#[derive(Debug)]
pub enum StreamError {
    UnknownStation,
    UnsafeEndpoint,
    InvalidPlaylist,
    OversizedPlaylist,
    TooManyEntries,
    TooManyHops,
    MissingLocation,
    HttpStatus(u16),
    UnsupportedContentType,
    InvalidIcyInterval,
    TransportUnavailable,
    Io(io::Error),
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for StreamError {}

impl From<io::Error> for StreamError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// A checked URL bound to exactly one curated station's broadcaster.
/// The private fields prevent accidental construction from arbitrary URLs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovedEndpoint {
    station_id: &'static str,
    url: String,
}

impl ApprovedEndpoint {
    pub fn new(station_id: &str, url: &str) -> Result<Self, StreamError> {
        let station = station_by_id(station_id).ok_or(StreamError::UnknownStation)?;
        if url.len() > 2048 || !url.starts_with("https://") {
            return Err(StreamError::UnsafeEndpoint);
        }
        let (authority, path) = url[8..]
            .split_once('/')
            .ok_or(StreamError::UnsafeEndpoint)?;
        if authority.is_empty()
            || path.is_empty()
            || !authority.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-' || b == b':'
            })
            || path.bytes().any(|b| {
                !b.is_ascii_graphic()
                    || b == b'\\'
                    || b == b'?'
                    || b == b'#'
                    || b == b'%'
                    || b == b'@'
            })
            || path.split('/').any(|part| part == "." || part == "..")
        {
            return Err(StreamError::UnsafeEndpoint);
        }

        let allowed = if station.id == "skylab" {
            authority == "uksoutha.streaming.broadcast.radio:29690"
                && path.starts_with("skylab-radio-limited")
        } else {
            let host = authority.strip_suffix(":443").unwrap_or(authority);
            !host.contains(':')
                && (host == "somafm.com" || host.ends_with(".somafm.com"))
                && host.split('.').all(|label| {
                    !label.is_empty()
                        && !label.starts_with('-')
                        && !label.ends_with('-')
                        && label.bytes().all(|byte| {
                            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                        })
                })
        };
        if !allowed {
            return Err(StreamError::UnsafeEndpoint);
        }
        Ok(Self {
            station_id: station.id,
            url: url.to_owned(),
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn station_id(&self) -> &'static str {
        self.station_id
    }
}

/// A transport is responsible for TLS verification, DNS/address safety,
/// redirect refusal, connection/idle timeouts, and prompt cancellation.
/// Core resolvers revalidate all returned absolute redirect/playlist URLs.
pub trait RadioHttp {
    type Body: Read;

    fn get(&self, target: &ApprovedEndpoint) -> Result<HttpResponse<Self::Body>, StreamError>;
}

/// Response body MUST be streamed; audio responses cannot be buffered entirely.
pub struct HttpResponse<R: Read> {
    pub status: u16,
    pub content_type: Option<String>,
    pub redirect_location: Option<String>,
    pub icy_metaint: Option<String>,
    pub body: R,
}

pub struct OpenRadioStream<R: Read> {
    pub endpoint: ApprovedEndpoint,
    pub content_type: Option<String>,
    pub reader: IcyAudioReader<R>,
}

/// Parse official M3U/extended-M3U playlists. Fail closed on unapproved lines.
pub fn parse_m3u(station_id: &str, body: &[u8]) -> Result<Vec<ApprovedEndpoint>, StreamError> {
    if body.len() > MAX_PLAYLIST_BYTES {
        return Err(StreamError::OversizedPlaylist);
    }
    let text = std::str::from_utf8(body).map_err(|_| StreamError::InvalidPlaylist)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut result = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        result.push(ApprovedEndpoint::new(station_id, trimmed)?);
        if result.len() > MAX_PLAYLIST_ENTRIES {
            return Err(StreamError::TooManyEntries);
        }
    }
    if result.is_empty() {
        return Err(StreamError::InvalidPlaylist);
    }
    Ok(result)
}

fn is_playlist(url: &str, content_type: Option<&str>) -> bool {
    let mime = content_type
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim();
    url.ends_with(".m3u")
        || ["audio/x-mpegurl", "audio/mpegurl", "application/x-mpegurl"]
            .iter()
            .any(|value| mime.eq_ignore_ascii_case(value))
}

fn is_audio(content_type: Option<&str>) -> bool {
    let mime = content_type
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    mime.starts_with("audio/") || mime == "application/octet-stream"
}

pub fn icy_interval(value: Option<&str>) -> Result<Option<usize>, StreamError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let number: usize = value
        .trim()
        .parse()
        .map_err(|_| StreamError::InvalidIcyInterval)?;
    if !(1..=MAX_ICY_INTERVAL).contains(&number) {
        return Err(StreamError::InvalidIcyInterval);
    }
    Ok(Some(number))
}

/// Resolve a curated station to a bounded audio response, using only
/// publisher-allowlisted endpoints. This routine never decodes sound.
pub fn open_station<C: RadioHttp>(
    client: &C,
    station_id: &str,
) -> Result<OpenRadioStream<C::Body>, StreamError> {
    let station = station_by_id(station_id).ok_or(StreamError::UnknownStation)?;
    let mut next = ApprovedEndpoint::new(station_id, station.playlist_url)?;
    let mut playlist_seen = false;

    for _ in 0..MAX_REQUEST_HOPS {
        let response = client.get(&next)?;
        if matches!(response.status, 301 | 302 | 303 | 307 | 308) {
            let location = response
                .redirect_location
                .as_deref()
                .ok_or(StreamError::MissingLocation)?;
            next = ApprovedEndpoint::new(station_id, location)?;
            continue;
        }
        if response.status != 200 {
            return Err(StreamError::HttpStatus(response.status));
        }
        if is_playlist(next.url(), response.content_type.as_deref()) {
            if playlist_seen {
                return Err(StreamError::TooManyHops);
            }
            playlist_seen = true;
            let mut document = Vec::new();
            response
                .body
                .take((MAX_PLAYLIST_BYTES + 1) as u64)
                .read_to_end(&mut document)?;
            let mut candidates = parse_m3u(station_id, &document)?;
            next = candidates.remove(0);
            continue;
        }
        if !is_audio(response.content_type.as_deref()) {
            return Err(StreamError::UnsupportedContentType);
        }
        let interval = icy_interval(response.icy_metaint.as_deref())?;
        return Ok(OpenRadioStream {
            endpoint: next,
            content_type: response.content_type,
            reader: IcyAudioReader::new(response.body, interval),
        });
    }
    Err(StreamError::TooManyHops)
}

/// Decoded ICY metadata is never mixed into the audio byte stream.
/// No network calls or audio decoding occur here.
pub struct IcyAudioReader<R: Read> {
    inner: R,
    interval: Option<usize>,
    remaining_audio: usize,
    last_title: Option<String>,
}

impl<R: Read> IcyAudioReader<R> {
    pub fn new(inner: R, interval: Option<usize>) -> Self {
        Self {
            inner,
            interval,
            remaining_audio: interval.unwrap_or(0),
            last_title: None,
        }
    }

    pub fn take_last_title(&mut self) -> Option<String> {
        self.last_title.take()
    }

    fn consume_metadata(&mut self) -> io::Result<()> {
        let mut length = [0u8; 1];
        self.inner.read_exact(&mut length)?;
        let bytes = usize::from(length[0]) * 16; // ICY length is at most 4080.
        let mut metadata = vec![0u8; bytes];
        self.inner.read_exact(&mut metadata)?;
        self.last_title = parse_icy_title(&metadata);
        self.remaining_audio = self.interval.unwrap_or(0);
        Ok(())
    }
}

impl<R: Read> Read for IcyAudioReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let cap = buffer.len().min(MAX_AUDIO_READ);
        if self.interval.is_some() {
            if self.remaining_audio == 0 {
                self.consume_metadata()?;
            }
            let count = self
                .inner
                .read(&mut buffer[..cap.min(self.remaining_audio)])?;
            self.remaining_audio -= count;
            Ok(count)
        } else {
            self.inner.read(&mut buffer[..cap])
        }
    }
}

fn parse_icy_title(metadata: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(metadata);
    let start = text.find("StreamTitle='")? + "StreamTitle='".len();
    let rest = &text[start..];
    let end = rest.find("';")?;
    let sanitized: String = rest[..end]
        .chars()
        .filter(|ch| !ch.is_control())
        .take(MAX_TITLE_BYTES)
        .collect();
    if sanitized.is_empty() {
        None
    } else {
        Some(sanitized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    struct Fixture {
        url: &'static str,
        status: u16,
        mime: &'static str,
        redirect: Option<&'static str>,
        metaint: Option<&'static str>,
        body: Vec<u8>,
    }

    struct FakeHttp(Vec<Fixture>);

    impl RadioHttp for FakeHttp {
        type Body = Cursor<Vec<u8>>;

        fn get(&self, target: &ApprovedEndpoint) -> Result<HttpResponse<Self::Body>, StreamError> {
            let fixture = self
                .0
                .iter()
                .find(|entry| entry.url == target.url())
                .ok_or(StreamError::TransportUnavailable)?;
            Ok(HttpResponse {
                status: fixture.status,
                content_type: Some(fixture.mime.into()),
                redirect_location: fixture.redirect.map(str::to_string),
                icy_metaint: fixture.metaint.map(str::to_string),
                body: Cursor::new(fixture.body.clone()),
            })
        }
    }

    #[test]
    fn tls_and_station_origin_are_enforced() {
        for url in [
            "http://somafm.com/m3u/groovesalad.m3u",
            "https://127.0.0.1/audio",
            "https://somafm.com.evil.test/audio",
            "https://evil.test@somafm.com/stream",
            "https://somafm.com:8080/audio",
            "https://somafm.com/%2e%2e/secret",
            "https://somafm.com/../secret",
            "https://somafm.com/stream?redirect=localhost",
            "https://uksoutha.streaming.broadcast.radio:29690/skylab-radio-limited",
        ] {
            assert!(ApprovedEndpoint::new("groovesalad", url).is_err(), "{url}");
        }
        assert!(ApprovedEndpoint::new(
            "skylab",
            "https://uksoutha.streaming.broadcast.radio:29690/skylab-radio-limited"
        )
        .is_ok());
        assert!(ApprovedEndpoint::new(
            "groovesalad",
            "https://ice5.somafm.com/groovesalad-128-mp3"
        )
        .is_ok());
    }

    #[test]
    fn m3u_limits_and_comments_are_handled() {
        let playlist = b"#EXTM3U\r\n#EXTINF:-1,Groove Salad\r\nhttps://ice5.somafm.com/groovesalad-128-mp3\r\n";
        let parsed = parse_m3u("groovesalad", playlist).unwrap();
        assert_eq!(parsed.len(), 1);
        assert!(parsed[0].url().contains("ice5"));
        assert!(matches!(
            parse_m3u("groovesalad", &vec![b'a'; MAX_PLAYLIST_BYTES + 1]),
            Err(StreamError::OversizedPlaylist)
        ));
        assert!(parse_m3u("groovesalad", b"#EXTM3U\n# comment").is_err());
        assert!(parse_m3u("groovesalad", b"https://example.com/steal").is_err());
    }

    #[test]
    fn playlist_resolves_only_to_allowed_audio_response() {
        let station_url = "https://somafm.com/m3u/groovesalad.m3u";
        let ice = "https://ice5.somafm.com/groovesalad-128-mp3";
        let fixture = FakeHttp(vec![
            Fixture {
                url: station_url,
                status: 200,
                mime: "audio/x-mpegurl",
                redirect: None,
                metaint: None,
                body: format!("#EXTM3U\n{ice}\n").into_bytes(),
            },
            Fixture {
                url: ice,
                status: 200,
                mime: "audio/mpeg",
                redirect: None,
                metaint: Some("4"),
                body: {
                    let mut body = b"abcd".to_vec();
                    let mut meta = [0u8; 16];
                    meta.copy_from_slice(b"StreamTitle='X';");
                    body.push(1);
                    body.extend_from_slice(&meta);
                    body.extend_from_slice(b"efgh");
                    body
                },
            },
        ]);
        let mut opened = open_station(&fixture, "groovesalad").unwrap();
        assert_eq!(opened.endpoint.url(), ice);
        let mut audio = [0u8; 4];
        assert_eq!(opened.reader.read(&mut audio).unwrap(), 4);
        assert_eq!(&audio, b"abcd");
        assert_eq!(opened.reader.read(&mut audio).unwrap(), 4);
        assert_eq!(&audio, b"efgh");
        assert_eq!(opened.reader.take_last_title(), Some("X".into()));
    }

    #[test]
    fn disallowed_redirect_cannot_be_requested() {
        let fixture = FakeHttp(vec![Fixture {
            url: "https://somafm.com/m3u/groovesalad.m3u",
            status: 302,
            mime: "text/plain",
            redirect: Some("https://127.0.0.1/internal"),
            metaint: None,
            body: vec![],
        }]);
        assert!(matches!(
            open_station(&fixture, "groovesalad"),
            Err(StreamError::UnsafeEndpoint)
        ));
    }

    #[test]
    fn invalid_icy_interval_and_content_are_rejected() {
        assert!(matches!(
            icy_interval(Some("0")),
            Err(StreamError::InvalidIcyInterval)
        ));
        assert!(matches!(
            icy_interval(Some("abc")),
            Err(StreamError::InvalidIcyInterval)
        ));
        assert!(!is_audio(Some("text/html")));
    }

    #[test]
    fn truncated_metadata_is_a_stream_error() {
        let data = b"ABCD\x02too-short".to_vec();
        let mut reader = IcyAudioReader::new(Cursor::new(data), Some(4));
        let mut buf = [0u8; 4];
        assert_eq!(reader.read(&mut buf).unwrap(), 4);
        assert!(reader.read(&mut buf).is_err());
    }
}
