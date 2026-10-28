//! Curated, receive-only internet radio fallback.
//!
//! Source selection is pure and testable. Actual SDR discovery and station
//! playback are independently gated; TCP reachability is not stream validation.

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// A publisher-documented stream directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InternetStation {
    pub id: &'static str,
    pub name: &'static str,
    /// Broadcaster that operates or publishes the station stream.
    pub broadcaster: &'static str,
    pub genre: &'static str,
    /// ISO 639-1 language tag for the station's primary programming.
    pub language: &'static str,
    /// ISO 3166-1 alpha-2 country code for the broadcaster.
    pub country: &'static str,
    /// Short, operator-visible catalog grouping; not a broadcast band.
    pub category: &'static str,
    pub playlist_url: &'static str,
    pub publisher_reference_url: &'static str,
    /// The publisher explicitly documents playback through third-party radio players.
    pub third_party_player_support: bool,
    /// Date the publisher's third-party playback guidance was reviewed.
    pub rights_reviewed_date: &'static str,
    pub nominal_kbps: u16,
    /// Publisher-declared codec hint; runtime validation records decoded format.
    pub format: &'static str,
}

/// Curated streams whose publishers document use in third-party radio players.
/// This guidance covers end-user playback only; it is not redistribution or
/// relay permission. Stations lacking this evidence stay out of this directory.
pub const CURATED_STATIONS: [InternetStation; 15] = [
    InternetStation {
        id: "skylab",
        name: "Skylab Radio (UK)",
        broadcaster: "Skylab Radio",
        language: "en",
        country: "GB",
        genre: "Chillout / lounge / ambient",
        category: "CHILL",
        playlist_url: "https://uksoutha.streaming.broadcast.radio:29690/skylab-radio-limited",
        publisher_reference_url: "https://www.skylabradio.com/listen",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 0,
        format: "Streaming audio (verify at runtime)",
    },
    InternetStation {
        id: "wcrb",
        name: "WCRB Classical (Boston)",
        broadcaster: "GBH",
        language: "en",
        country: "US",
        genre: "Classical",
        category: "CLASSICAL",
        playlist_url: "https://wgbh-live.streamguys1.com/classical-hi/",
        publisher_reference_url: "https://www.wgbh.org/help/how-to-access-live-radio-streams",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 0,
        format: "AAC",
    },
    InternetStation {
        id: "gbh-news",
        name: "GBH 89.7 News Radio",
        broadcaster: "GBH",
        language: "en",
        country: "US",
        genre: "NPR / local news and public affairs",
        category: "NEWS",
        playlist_url: "https://wgbh-live.streamguys1.com/wgbh",
        publisher_reference_url: "https://www.wgbh.org/help/how-to-access-live-radio-streams",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 0,
        format: "Streaming audio (verify at runtime)",
    },
    InternetStation {
        id: "kexp",
        name: "KEXP 90.3 FM",
        broadcaster: "KEXP",
        language: "en",
        country: "US",
        genre: "Independent / alternative / world music",
        category: "INDIE",
        playlist_url: "https://kexp.streamguys1.com/kexp160.aac",
        publisher_reference_url: "https://www.kexp.org/mobile/kexp-livestreams/",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 160,
        format: "AAC",
    },
    InternetStation {
        id: "radio-paradise",
        name: "Radio Paradise Main Mix",
        broadcaster: "Radio Paradise",
        language: "en",
        country: "US",
        genre: "Eclectic commercial-free mix",
        category: "MIXED",
        playlist_url: "https://stream.radioparadise.com/mp3-128",
        publisher_reference_url: "https://radioparadise.com/listen/stream-links",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "wpr-news",
        name: "WPR News",
        broadcaster: "Wisconsin Public Radio",
        language: "en",
        country: "US",
        genre: "News / talk / public affairs",
        category: "NEWS",
        playlist_url: "https://wpr-ice.streamguys1.com/wpr-ideas-mp3-64",
        publisher_reference_url: "https://www.wpr.org/ways-to-listen",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 64,
        format: "MP3",
    },
    InternetStation {
        id: "wpr-music",
        name: "WPR Music",
        broadcaster: "Wisconsin Public Radio",
        language: "en",
        country: "US",
        genre: "Music / culture",
        category: "MIXED",
        playlist_url: "https://wpr-ice.streamguys1.com/wpr-music-mp3-96",
        publisher_reference_url: "https://www.wpr.org/ways-to-listen",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 96,
        format: "MP3",
    },
    InternetStation {
        id: "radio-swiss-jazz",
        name: "Radio Swiss Jazz",
        broadcaster: "Swiss Satellite Radio",
        language: "de",
        country: "CH",
        genre: "Jazz / blues / soul",
        category: "JAZZ",
        playlist_url: "https://stream.srg-ssr.ch/srgssr/rsj/mp3/128",
        publisher_reference_url: "https://www.radioswissjazz.ch/de/empfang/internet",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "radio-swiss-classic",
        name: "Radio Swiss Classic",
        broadcaster: "Swiss Satellite Radio",
        language: "de",
        country: "CH",
        genre: "Classical",
        category: "CLASSICAL",
        playlist_url: "https://stream.srg-ssr.ch/srgssr/rsc_de/mp3/128",
        publisher_reference_url: "https://www.radioswissclassic.ch/de/empfang/internet",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "all-classical",
        name: "All Classical Radio",
        broadcaster: "All Classical Public Media",
        language: "en",
        country: "US",
        genre: "Classical",
        category: "CLASSICAL",
        playlist_url: "https://allclassical.streamguys1.com/ac128kmp3",
        publisher_reference_url: "https://www.allclassical.org/contact/contact-our-technical-team/",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "wfmt",
        name: "WFMT Classical",
        broadcaster: "WFMT",
        language: "en",
        country: "US",
        genre: "Classical",
        category: "CLASSICAL",
        playlist_url: "https://wfmt-live.streamguys1.com/wfmt256-aac",
        publisher_reference_url: "https://www.wfmt.com/how-to-listen/",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 256,
        format: "AAC",
    },
    InternetStation {
        id: "kfuo",
        name: "KFUO Radio",
        broadcaster: "KFUO Radio",
        language: "en",
        country: "US",
        genre: "Faith / classical / talk",
        category: "MIXED",
        playlist_url: "https://kfuo.streamguys1.com/kfuo",
        publisher_reference_url: "https://www.kfuo.org/listening-options/",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 0,
        format: "Streaming audio (verify at runtime)",
    },
    InternetStation {
        id: "wrvo-news",
        name: "WRVO 1 NPR News",
        broadcaster: "WRVO Public Media",
        language: "en",
        country: "US",
        genre: "NPR News / local public affairs",
        category: "NEWS",
        playlist_url:
            "https://playerservices.streamtheworld.com/api/livestream-redirect/WRVOFM.mp3",
        publisher_reference_url: "https://www.wrvo.org/listen",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 0,
        format: "MP3",
    },
    InternetStation {
        id: "wrvo-prx",
        name: "WRVO 2 PRX Remix",
        broadcaster: "WRVO Public Media",
        language: "en",
        country: "US",
        genre: "Spoken word / PRX Remix",
        category: "TALK",
        playlist_url:
            "https://playerservices.streamtheworld.com/api/livestream-redirect/WRVOHD2.mp3",
        publisher_reference_url: "https://www.wrvo.org/digital-streaming",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 0,
        format: "MP3",
    },
    InternetStation {
        id: "wrvo-bbc",
        name: "WRVO 3 BBC World Service",
        broadcaster: "WRVO Public Media",
        language: "en",
        country: "US",
        genre: "International news / BBC World Service",
        category: "NEWS",
        playlist_url:
            "https://playerservices.streamtheworld.com/api/livestream-redirect/WRVOHD3.mp3",
        publisher_reference_url: "https://www.wrvo.org/digital-streaming",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 0,
        format: "MP3",
    },
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioSource {
    HardwareSdr,
    InternetDirectory,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpReachability {
    Reachable,
    Unreachable,
    NotChecked,
}

/// Distinguish a detected hardware device from a simulation fixture.
pub trait RadioDiscovery {
    fn real_receiver_available(&self) -> bool;
}

/// Availability checks may resolve and connect to *approved radio provider*
/// hosts only. Do not use untrusted remote station-provided hostnames.
pub trait RadioConnectivity {
    fn approved_provider_reachable(&self) -> IpReachability;
}

pub fn select_source<D: RadioDiscovery, C: RadioConnectivity>(
    device: &D,
    connectivity: &C,
) -> RadioSource {
    if device.real_receiver_available() {
        RadioSource::HardwareSdr
    } else if connectivity.approved_provider_reachable() == IpReachability::Reachable {
        RadioSource::InternetDirectory
    } else {
        RadioSource::Offline
    }
}

/// The core currently has no physical SDR backend. Never report the simulator
/// as a receiver: the eventual SoapySDR enumerator replaces this implementation.
pub struct NoHardwareEnumerator;

impl RadioDiscovery for NoHardwareEnumerator {
    fn real_receiver_available(&self) -> bool {
        false
    }
}

pub struct UncheckedConnectivity;
impl RadioConnectivity for UncheckedConnectivity {
    fn approved_provider_reachable(&self) -> IpReachability {
        IpReachability::NotChecked
    }
}

/// Bounded TCP connectivity check to the publisher's HTTPS endpoint.
/// It proves only that a TCP route exists, NOT that HTTPS, the playlist,
/// a codec or the station stream is usable.
pub struct PublisherTcpCheck {
    pub timeout: Duration,
}

impl Default for PublisherTcpCheck {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
        }
    }
}

impl RadioConnectivity for PublisherTcpCheck {
    fn approved_provider_reachable(&self) -> IpReachability {
        let timeout = self.timeout.min(Duration::from_secs(3));
        let Ok(addresses) = ("wgbh-live.streamguys1.com", 443).to_socket_addrs() else {
            return IpReachability::Unreachable;
        };
        // Bound probe time even if DNS returns multiple addresses.
        if let Some(address) = addresses
            .take(2)
            .find(|address| TcpStream::connect_timeout(address, timeout).is_ok())
        {
            let _ = address;
            IpReachability::Reachable
        } else {
            IpReachability::Unreachable
        }
    }
}

/// Station-specific TCP reachability probe. Host and port are fixed by curated
/// station ID, never inferred from a user-provided URL. Does not prove audio.
pub fn station_tcp_reachable(id: &str, timeout: Duration) -> Option<IpReachability> {
    let (host, port) = station_tcp_target(id)?;
    let timeout = timeout.min(Duration::from_secs(3));
    let Ok(addrs) = (host, port).to_socket_addrs() else {
        return Some(IpReachability::Unreachable);
    };
    let reachable = addrs
        .take(2)
        .any(|addr| TcpStream::connect_timeout(&addr, timeout).is_ok());
    Some(if reachable {
        IpReachability::Reachable
    } else {
        IpReachability::Unreachable
    })
}

/// Resolve only a curated station ID to its publisher-approved HTTPS host.
/// Keeping this derived from the catalog prevents the probe allowlist from
/// silently omitting a station or drifting when a publisher endpoint changes.
fn station_tcp_target(id: &str) -> Option<(&str, u16)> {
    let station = station_by_id(id)?;
    let endpoint = station.playlist_url.strip_prefix("https://")?;
    let authority = endpoint.split('/').next()?;
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, port.parse().ok()?),
        None => (authority, 443),
    };
    (!host.is_empty()).then_some((host, port))
}

pub fn curated_stations() -> &'static [InternetStation] {
    &CURATED_STATIONS
}

pub fn station_by_id(id: &str) -> Option<&'static InternetStation> {
    curated_stations().iter().find(|station| station.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct MockHardware(bool);
    impl RadioDiscovery for MockHardware {
        fn real_receiver_available(&self) -> bool {
            self.0
        }
    }
    struct MockIp(IpReachability);
    impl RadioConnectivity for MockIp {
        fn approved_provider_reachable(&self) -> IpReachability {
            self.0
        }
    }

    #[test]
    fn lookup_is_exact_and_allowlisted() {
        assert_eq!(
            station_by_id("skylab").map(|s| s.name),
            Some("Skylab Radio (UK)")
        );
        assert!(station_by_id("https://attacker.test").is_none());
        assert_eq!(
            station_tcp_reachable("https://attacker.test", Duration::ZERO),
            None
        );
    }
    #[test]
    fn sdr_precedes_network() {
        assert_eq!(
            select_source(&MockHardware(true), &MockIp(IpReachability::Unreachable)),
            RadioSource::HardwareSdr
        );
    }
    #[test]
    fn ip_fallback_requires_reachable_provider() {
        assert_eq!(
            select_source(&MockHardware(false), &MockIp(IpReachability::Reachable)),
            RadioSource::InternetDirectory
        );
        assert_eq!(
            select_source(&MockHardware(false), &MockIp(IpReachability::Unreachable)),
            RadioSource::Offline
        );
        assert_eq!(
            select_source(&MockHardware(false), &UncheckedConnectivity),
            RadioSource::Offline
        );
    }
    #[test]
    fn curated_urls_have_publisher_provenance() {
        assert_eq!(curated_stations().len(), 15);
        for (id, endpoint) in [
            (
                "wpr-news",
                "https://wpr-ice.streamguys1.com/wpr-ideas-mp3-64",
            ),
            (
                "wpr-music",
                "https://wpr-ice.streamguys1.com/wpr-music-mp3-96",
            ),
            (
                "radio-swiss-jazz",
                "https://stream.srg-ssr.ch/srgssr/rsj/mp3/128",
            ),
            (
                "radio-swiss-classic",
                "https://stream.srg-ssr.ch/srgssr/rsc_de/mp3/128",
            ),
            (
                "all-classical",
                "https://allclassical.streamguys1.com/ac128kmp3",
            ),
            ("wfmt", "https://wfmt-live.streamguys1.com/wfmt256-aac"),
            ("kfuo", "https://kfuo.streamguys1.com/kfuo"),
        ] {
            assert_eq!(station_by_id(id).unwrap().playlist_url, endpoint);
        }
        let mut ids = std::collections::BTreeSet::new();
        for station in curated_stations() {
            assert!(ids.insert(station.id), "duplicate station ID");
            assert!(
                station_tcp_target(station.id).is_some(),
                "curated station has no safe TCP probe target: {}",
                station.id
            );
            assert!(!station.broadcaster.trim().is_empty());
            assert_eq!(station.language.len(), 2, "language tag: {}", station.id);
            assert!(station
                .language
                .bytes()
                .all(|byte| byte.is_ascii_lowercase()));
            assert_eq!(station.country.len(), 2, "country code: {}", station.id);
            assert!(station
                .country
                .bytes()
                .all(|byte| byte.is_ascii_uppercase()));
            assert!([
                "CHILL",
                "AMBIENT",
                "ELECTRONIC",
                "RETRO",
                "CLASSICAL",
                "INDIE",
                "FOLK",
                "ROCK",
                "ROOTS",
                "JAZZ",
                "WORLD",
                "HIPHOP",
                "DANCE",
                "POP",
                "LOUNGE",
                "NEWS",
                "MIXED",
                "TALK"
            ]
            .contains(&station.category));
            assert!(!station.genre.is_empty());
            assert!(station.playlist_url.starts_with("https://"));
            assert!(station.publisher_reference_url.starts_with("https://"));
            if station.id == "skylab" {
                assert!(station
                    .playlist_url
                    .starts_with("https://uksoutha.streaming.broadcast.radio:29690/"));
                assert_eq!(
                    station.publisher_reference_url,
                    "https://www.skylabradio.com/listen"
                );
            } else if station.id == "wcrb" {
                assert_eq!(
                    station.playlist_url,
                    "https://wgbh-live.streamguys1.com/classical-hi/"
                );
                assert_eq!(station.format, "AAC");
            } else if station.id == "gbh-news" {
                assert_eq!(
                    station.playlist_url,
                    "https://wgbh-live.streamguys1.com/wgbh"
                );
                assert!(station.genre.starts_with("NPR /"));
            }
            assert!(station.third_party_player_support);
            assert_eq!(station.rights_reviewed_date, "2026-10-28");
            assert!(!station.name.is_empty());
        }
    }

    #[test]
    fn station_probe_target_comes_from_curated_https_endpoint() {
        assert_eq!(
            station_tcp_target("wcrb"),
            Some(("wgbh-live.streamguys1.com", 443))
        );
        assert_eq!(
            station_tcp_target("skylab"),
            Some(("uksoutha.streaming.broadcast.radio", 29690))
        );
        assert_eq!(station_tcp_target("unknown"), None);
        assert_eq!(station_tcp_target("https://attacker.test"), None);
    }
}
