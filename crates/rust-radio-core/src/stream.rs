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
    UnsupportedHls,
    InvalidIcyInterval,
    TransportUnavailable,
    Io(io::Error),
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedHls => f.write_str("HLS radio transport is not supported"),
            error => write!(f, "{error:?}"),
        }
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
        } else if station.id == "wcrb" {
            authority == "wgbh-live.streamguys1.com" && path == "classical-hi/"
        } else if station.id == "gbhnews" {
            authority == "wgbh-live.streamguys1.com" && path == "wgbh"
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

/// SomaFM's HTTPS-served playlists still contain legacy HTTP stream links.
/// Request only their TLS equivalents, subject to the same publisher/port/path
/// checks as every other endpoint. Never send a plaintext request or downgrade.
fn playlist_endpoint(station_id: &str, url: &str) -> Result<ApprovedEndpoint, StreamError> {
    let station = station_by_id(station_id).ok_or(StreamError::UnknownStation)?;
    if station
        .publisher_reference_url
        .starts_with("https://somafm.com/")
    {
        if let Some(rest) = url.strip_prefix("http://") {
            return ApprovedEndpoint::new(station_id, &format!("https://{rest}"));
        }
    }
    ApprovedEndpoint::new(station_id, url)
}

/// Parse official M3U/extended-M3U playlists. Fail closed on unapproved lines.
pub fn parse_m3u(station_id: &str, body: &[u8]) -> Result<Vec<ApprovedEndpoint>, StreamError> {
    parse_m3u_with(body, |url| playlist_endpoint(station_id, url))
}

/// Parse bounded M3U entries with transport-owned URL resolution.
pub fn parse_m3u_with<T>(
    body: &[u8],
    mut resolve: impl FnMut(&str) -> Result<T, StreamError>,
) -> Result<Vec<T>, StreamError> {
    if body.len() > MAX_PLAYLIST_BYTES {
        return Err(StreamError::OversizedPlaylist);
    }
    let text = std::str::from_utf8(body).map_err(|_| StreamError::InvalidPlaylist)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut result = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#EXT-X-") {
            return Err(StreamError::UnsupportedHls);
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        result.push(resolve(trimmed)?);
        if result.len() > MAX_PLAYLIST_ENTRIES {
            return Err(StreamError::TooManyEntries);
        }
    }
    if result.is_empty() {
        return Err(StreamError::InvalidPlaylist);
    }
    Ok(result)
}

/// Parse bounded PLS v2 playlists. Every FileN URL must be publisher-approved;
/// titles and lengths are inert metadata. Entries are returned in numeric order.
pub fn parse_pls(station_id: &str, body: &[u8]) -> Result<Vec<ApprovedEndpoint>, StreamError> {
    parse_pls_with(body, |url| playlist_endpoint(station_id, url))
}

/// Parse bounded PLS entries with transport-owned URL resolution.
pub fn parse_pls_with<T>(
    body: &[u8],
    mut resolve: impl FnMut(&str) -> Result<T, StreamError>,
) -> Result<Vec<T>, StreamError> {
    if body.len() > MAX_PLAYLIST_BYTES {
        return Err(StreamError::OversizedPlaylist);
    }
    let text = std::str::from_utf8(body).map_err(|_| StreamError::InvalidPlaylist)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut header_seen = false;
    let mut declared_entries = None;
    let mut version_seen = false;
    // FileN is already a bounded numeric index; no tree nodes are needed.
    let mut entries: [Option<T>; MAX_PLAYLIST_ENTRIES] = std::array::from_fn(|_| None);
    let mut entry_count = 0;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if !header_seen {
            if !line.eq_ignore_ascii_case("[playlist]") {
                return Err(StreamError::InvalidPlaylist);
            }
            header_seen = true;
            continue;
        }
        let (key, value) = line.split_once('=').ok_or(StreamError::InvalidPlaylist)?;
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        match key.as_str() {
            "numberofentries" => {
                if declared_entries.is_some() {
                    return Err(StreamError::InvalidPlaylist);
                }
                let count = value
                    .parse::<usize>()
                    .map_err(|_| StreamError::InvalidPlaylist)?;
                if count > MAX_PLAYLIST_ENTRIES {
                    return Err(StreamError::TooManyEntries);
                }
                declared_entries = Some(count);
            }
            "version" => {
                if version_seen || value != "2" {
                    return Err(StreamError::InvalidPlaylist);
                }
                version_seen = true;
            }
            _ => {
                if let Some(index) = key.strip_prefix("file") {
                    let index = index
                        .parse::<usize>()
                        .map_err(|_| StreamError::InvalidPlaylist)?;
                    if index == 0 {
                        return Err(StreamError::InvalidPlaylist);
                    }
                    if index > MAX_PLAYLIST_ENTRIES {
                        return Err(StreamError::TooManyEntries);
                    }
                    let endpoint = resolve(value)?;
                    if entries[index - 1].replace(endpoint).is_some() {
                        return Err(StreamError::InvalidPlaylist);
                    }
                    entry_count += 1;
                }
            }
        }
    }
    if entry_count == 0 || declared_entries != Some(entry_count) {
        return Err(StreamError::InvalidPlaylist);
    }
    entries
        .into_iter()
        .take(entry_count)
        .collect::<Option<Vec<_>>>()
        .ok_or(StreamError::InvalidPlaylist)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaylistFormat {
    M3u,
    Pls,
    /// Segmented HLS needs a dedicated transport; never treat it as direct audio.
    Hls,
}

/// Shared detection for offline validation and sustained native playback.
pub fn playlist_format(url: &str, content_type: Option<&str>) -> Option<PlaylistFormat> {
    let mime = content_type
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let url = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .to_ascii_lowercase();
    if url.ends_with(".m3u8") || mime == "application/vnd.apple.mpegurl" {
        Some(PlaylistFormat::Hls)
    } else if matches!(
        mime.as_str(),
        "audio/x-scpls" | "audio/scpls" | "application/x-scpls"
    ) {
        Some(PlaylistFormat::Pls)
    } else if matches!(
        mime.as_str(),
        "audio/x-mpegurl" | "audio/mpegurl" | "application/x-mpegurl"
    ) {
        Some(PlaylistFormat::M3u)
    } else if url.ends_with(".pls") {
        Some(PlaylistFormat::Pls)
    } else if url.ends_with(".m3u") {
        Some(PlaylistFormat::M3u)
    } else {
        None
    }
}

pub fn parse_playlist(
    station_id: &str,
    body: &[u8],
    format: PlaylistFormat,
) -> Result<Vec<ApprovedEndpoint>, StreamError> {
    match format {
        PlaylistFormat::M3u => parse_m3u(station_id, body),
        PlaylistFormat::Pls => parse_pls(station_id, body),
        PlaylistFormat::Hls => Err(StreamError::UnsupportedHls),
    }
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
    if number == 0 {
        return Ok(None);
    }
    if number > MAX_ICY_INTERVAL {
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
        if let Some(format) = playlist_format(next.url(), response.content_type.as_deref()) {
            if format == PlaylistFormat::Hls {
                return Err(StreamError::UnsupportedHls);
            }
            if playlist_seen {
                return Err(StreamError::TooManyHops);
            }
            playlist_seen = true;
            let mut document = Vec::new();
            response
                .body
                .take((MAX_PLAYLIST_BYTES + 1) as u64)
                .read_to_end(&mut document)?;
            next = parse_playlist(station_id, &document, format)?
                .into_iter()
                .next()
                .ok_or(StreamError::InvalidPlaylist)?;
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
    fn pls_entries_are_bounded_allowlisted_and_sorted_by_number() {
        let playlist = "\u{feff}[PLAYLIST]\r\n; broadcaster playlist\r\nFile2=https://ice5.somafm.com/groovesalad-128-mp3\r\nTitle1=Groove=Salad\r\nLength1=-1\r\nfIlE1 = https://ice2.somafm.com/groovesalad-128-mp3\r\nNumberOfEntries=2\r\nVersion=2\r\n";
        let entries = parse_pls("groovesalad", playlist.as_bytes()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].url(),
            "https://ice2.somafm.com/groovesalad-128-mp3"
        );
        assert_eq!(
            entries[1].url(),
            "https://ice5.somafm.com/groovesalad-128-mp3"
        );
        assert!(matches!(
            parse_pls("groovesalad", &vec![b'a'; MAX_PLAYLIST_BYTES + 1]),
            Err(StreamError::OversizedPlaylist)
        ));
        for body in [
            "[playlist]\nNumberOfEntries=17\n",
            "[playlist]\nFile17=https://ice5.somafm.com/audio\nNumberOfEntries=1\n",
        ] {
            assert!(matches!(
                parse_pls("groovesalad", body.as_bytes()),
                Err(StreamError::TooManyEntries)
            ));
        }
    }

    #[test]
    fn pls_numeric_indexes_cover_capacity_and_reject_sparse_entries() {
        let mut body = format!("[playlist]\nNumberOfEntries={MAX_PLAYLIST_ENTRIES}\nVersion=2\n");
        for index in (1..=MAX_PLAYLIST_ENTRIES).rev() {
            body.push_str(&format!(
                "File{index}=https://ice5.somafm.com/radio-{index}\n"
            ));
        }
        let entries = parse_pls("groovesalad", body.as_bytes()).unwrap();
        assert_eq!(entries.len(), MAX_PLAYLIST_ENTRIES);
        for (index, endpoint) in entries.iter().enumerate() {
            assert_eq!(
                endpoint.url(),
                format!("https://ice5.somafm.com/radio-{}", index + 1)
            );
        }
        let sparse = body
            .replace("File8=https://ice5.somafm.com/radio-8\n", "")
            .replace(
                &format!("NumberOfEntries={MAX_PLAYLIST_ENTRIES}"),
                &format!("NumberOfEntries={}", MAX_PLAYLIST_ENTRIES - 1),
            );
        assert!(matches!(
            parse_pls("groovesalad", sparse.as_bytes()),
            Err(StreamError::InvalidPlaylist)
        ));
    }

    #[test]
    fn malformed_pls_counts_versions_and_duplicate_entries_are_rejected() {
        let audio = "https://ice5.somafm.com/groovesalad-128-mp3";
        for body in [
            String::new(),
            format!("File1={audio}\nNumberOfEntries=1"),
            "[playlist]\nNumberOfEntries=0".into(),
            format!("[playlist]\nFile1={audio}"),
            format!("[playlist]\nFile1={audio}\nNumberOfEntries=2"),
            format!("[playlist]\nFile2={audio}\nNumberOfEntries=1"),
            format!("[playlist]\nFile0={audio}\nNumberOfEntries=1"),
            format!("[playlist]\nFile1={audio}\nFile1={audio}\nNumberOfEntries=1"),
            format!("[playlist]\nFile1={audio}\nNumberOfEntries=1\nNumberOfEntries=1"),
            format!("[playlist]\nFile1={audio}\nNumberOfEntries=1\nVersion=3"),
            format!("[playlist]\nFile1={audio}\nNumberOfEntries=1\nVersion=2\nVersion=2"),
            format!("[playlist]\nFilex={audio}\nNumberOfEntries=1"),
        ] {
            assert!(
                matches!(
                    parse_pls("groovesalad", body.as_bytes()),
                    Err(StreamError::InvalidPlaylist)
                ),
                "{body}"
            );
        }
        assert!(matches!(
            parse_pls("groovesalad", b"[playlist]\n\xff"),
            Err(StreamError::InvalidPlaylist)
        ));
    }

    #[test]
    fn unapproved_pls_fallback_is_rejected_even_after_a_safe_first_entry() {
        for url in [
            "http://ice5.somafm.com:80/audio",
            "https://127.0.0.1/audio",
            "https://somafm.com.evil.test/audio",
        ] {
            let body = format!("[playlist]\nFile1=https://ice5.somafm.com/audio\nFile2={url}\nNumberOfEntries=2\nVersion=2");
            assert!(matches!(
                parse_pls("groovesalad", body.as_bytes()),
                Err(StreamError::UnsafeEndpoint)
            ));
        }
    }

    #[test]
    fn legacy_somafm_playlist_entries_are_upgraded_before_requesting() {
        let url = "http://ice6.somafm.com/groovesalad-128-mp3";
        assert!(ApprovedEndpoint::new("groovesalad", url).is_err());
        let m3u = parse_m3u("groovesalad", format!("#EXTM3U\n{url}").as_bytes()).unwrap();
        let pls = parse_pls(
            "groovesalad",
            format!("[playlist]\nFile1={url}\nNumberOfEntries=1\nVersion=2").as_bytes(),
        )
        .unwrap();
        for entries in [m3u, pls] {
            assert_eq!(
                entries[0].url(),
                "https://ice6.somafm.com/groovesalad-128-mp3"
            );
        }
        assert!(parse_m3u(
            "skylab",
            b"http://uksoutha.streaming.broadcast.radio:29690/skylab-radio-limited"
        )
        .is_err());
    }

    #[test]
    fn legacy_playlist_upgrade_never_relaxes_endpoint_safety() {
        for url in [
            "http://127.0.0.1/audio",
            "http://ice6.somafm.com.evil.test/audio",
            "http://ice6.somafm.com:80/audio",
            "http://ice6.somafm.com:8080/audio",
            "http://evil.test@ice6.somafm.com/audio",
            "http://ice6.somafm.com/../internal",
            "http://ice6.somafm.com/%2e%2e/internal",
            "http://ice6.somafm.com/audio?redirect=localhost",
        ] {
            assert!(
                matches!(
                    parse_m3u("groovesalad", url.as_bytes()),
                    Err(StreamError::UnsafeEndpoint)
                ),
                "{url}"
            );
        }
    }

    #[test]
    fn playlist_detection_distinguishes_pls_m3u_and_hls() {
        for (url, mime, expected) in [
            (
                "https://somafm.com/radio.PLS",
                None,
                Some(PlaylistFormat::Pls),
            ),
            (
                "https://somafm.com/radio",
                Some("Audio/X-SCPLS; charset=utf-8"),
                Some(PlaylistFormat::Pls),
            ),
            (
                "https://somafm.com/radio.m3u",
                Some("audio/x-scpls"),
                Some(PlaylistFormat::Pls),
            ),
            (
                "https://somafm.com/radio.m3u",
                None,
                Some(PlaylistFormat::M3u),
            ),
            (
                "https://somafm.com/radio",
                Some("application/x-mpegurl"),
                Some(PlaylistFormat::M3u),
            ),
            (
                "https://somafm.com/radio.M3U8",
                Some("audio/mpegurl"),
                Some(PlaylistFormat::Hls),
            ),
            (
                "https://somafm.com/radio",
                Some("application/vnd.apple.mpegurl"),
                Some(PlaylistFormat::Hls),
            ),
            ("https://somafm.com/radio", Some("audio/mpeg"), None),
        ] {
            assert_eq!(playlist_format(url, mime), expected);
        }
    }

    #[test]
    fn hls_manifests_are_never_treated_as_direct_station_playlists() {
        for body in [
            b"#EXTM3U\n#EXT-X-VERSION:3\nhttps://ice5.somafm.com/segment.ts".as_slice(),
            b"#EXTM3U\nhttps://ice5.somafm.com/audio\n#EXT-X-STREAM-INF:BANDWIDTH=128000",
        ] {
            assert!(matches!(
                parse_m3u("groovesalad", body),
                Err(StreamError::UnsupportedHls)
            ));
        }
        assert!(matches!(
            parse_playlist("groovesalad", b"", PlaylistFormat::Hls),
            Err(StreamError::UnsupportedHls)
        ));
    }

    #[test]
    fn redirected_pls_resolves_to_approved_audio() {
        let ice = "https://ice5.somafm.com/groovesalad-128-mp3";
        let fixture = FakeHttp(vec![
            Fixture {
                url: "https://somafm.com/m3u/groovesalad.m3u",
                status: 302,
                mime: "text/plain",
                redirect: Some("https://somafm.com/groovesalad.pls"),
                metaint: None,
                body: vec![],
            },
            Fixture {
                url: "https://somafm.com/groovesalad.pls",
                status: 200,
                mime: "audio/x-scpls",
                redirect: None,
                metaint: None,
                body: format!("[playlist]\nFile1={ice}\nNumberOfEntries=1\nVersion=2").into_bytes(),
            },
            Fixture {
                url: ice,
                status: 200,
                mime: "audio/mpeg",
                redirect: None,
                metaint: None,
                body: b"audio".to_vec(),
            },
        ]);
        let mut stream = open_station(&fixture, "groovesalad").unwrap();
        assert_eq!(stream.endpoint.url(), ice);
        let mut audio = String::new();
        stream.reader.read_to_string(&mut audio).unwrap();
        assert_eq!(audio, "audio");
    }

    #[test]
    fn resolver_rejects_hls_without_fetching_segments() {
        for mime in ["application/vnd.apple.mpegurl", "audio/x-mpegurl"] {
            let fixture = FakeHttp(vec![Fixture {
                url: "https://somafm.com/m3u/groovesalad.m3u",
                status: 200,
                mime,
                redirect: None,
                metaint: None,
                body: b"#EXTM3U\n#EXT-X-VERSION:3\nhttps://ice5.somafm.com/segment.ts".to_vec(),
            }]);
            assert!(matches!(
                open_station(&fixture, "groovesalad"),
                Err(StreamError::UnsupportedHls)
            ));
        }
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
        assert_eq!(icy_interval(Some("0")).unwrap(), None);
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
