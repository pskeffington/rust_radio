//! Curated, receive-only internet radio directory.

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// A first-party, publisher-documented stream directory entry.
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
    /// Publisher page currently documents use through third-party radio players.
    pub third_party_player_support: bool,
    /// Date that third-party playback guidance was last reviewed.
    pub rights_reviewed_date: &'static str,
    pub nominal_kbps: u16,
    /// Publisher-declared codec hint; runtime validation records decoded format.
    pub format: &'static str,
}

/// Twenty selected publisher-documented internet streams. Third-party-player
/// guidance supports listener playback only; it is not redistribution or relay
/// permission. Decoder support and current availability require runtime checks.
pub const CURATED_STATIONS: [InternetStation; 20] = [
    InternetStation {
        id: "skylab",
        name: "Skylab Radio (UK)",
        broadcaster: "Skylab Radio",
        language: "en",
        country: "GB",
        genre: "Chillout / lounge / ambient",
        category: "CHILL",
        playlist_url: "https://stream.rcs.revma.com/7vs4sr22w3bwv.pls",
        publisher_reference_url: "https://www.skylabradio.com/listen",
        third_party_player_support: true,
        rights_reviewed_date: "2026-11-27",
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
    InternetStation {
        id: "wfmu",
        name: "WFMU Freeform Radio",
        broadcaster: "WFMU",
        language: "en",
        country: "US",
        genre: "Freeform / independent",
        category: "MIXED",
        playlist_url: "https://stream0.wfmu.org/freeform-128k",
        publisher_reference_url: "https://wfmu.org/playlists/D5",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "wfmu-drummer",
        name: "Give the Drummer Radio",
        broadcaster: "WFMU",
        language: "en",
        country: "US",
        genre: "Eclectic / global / specialty",
        category: "WORLD",
        playlist_url: "https://stream0.wfmu.org/drummer",
        publisher_reference_url: "https://wfmu.org/playlists/D5",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "wfmu-rock-soul",
        name: "WFMU Rock'n'Soul Radio",
        broadcaster: "WFMU",
        language: "en",
        country: "US",
        genre: "Rock / soul / rhythm and blues",
        category: "ROCK",
        playlist_url: "https://stream0.wfmu.org/rocknsoul",
        publisher_reference_url: "https://wfmu.org/playlists/D5",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "wfmu-sheena",
        name: "Sheena's Jungle Room",
        broadcaster: "WFMU",
        language: "en",
        country: "US",
        genre: "Vintage / tropical / global",
        category: "WORLD",
        playlist_url: "https://stream0.wfmu.org/sheena",
        publisher_reference_url: "https://wfmu.org/playlists/D5",
        third_party_player_support: true,
        rights_reviewed_date: "2026-10-28",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "kunc",
        name: "KUNC News",
        broadcaster: "KUNC",
        language: "en",
        country: "US",
        genre: "NPR News / public affairs",
        category: "NEWS",
        playlist_url: "https://ais-sa1.streamon.fm/7890_96k.mp3",
        publisher_reference_url: "https://www.kunc.org/listen",
        third_party_player_support: true,
        rights_reviewed_date: "2026-11-27",
        nominal_kbps: 96,
        format: "MP3",
    },
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpReachability {
    Reachable,
    Unreachable,
    NotChecked,
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
    fn curated_urls_have_publisher_provenance() {
        const EXPECTED_IDS: [&str; 20] = [
            "skylab",
            "wcrb",
            "gbh-news",
            "kexp",
            "radio-paradise",
            "wpr-news",
            "wpr-music",
            "radio-swiss-jazz",
            "radio-swiss-classic",
            "all-classical",
            "wfmt",
            "kfuo",
            "wrvo-news",
            "wrvo-prx",
            "wrvo-bbc",
            "wfmu",
            "wfmu-drummer",
            "wfmu-rock-soul",
            "wfmu-sheena",
            "kunc",
        ];
        assert_eq!(curated_stations().len(), 20);
        assert_eq!(
            curated_stations()
                .iter()
                .map(|station| station.id)
                .collect::<Vec<_>>(),
            EXPECTED_IDS
        );
        for retired in [
            "groovesalad",
            "secretagent",
            "kusc",
            "colorado-sound",
            "kuaf",
        ] {
            assert!(
                station_by_id(retired).is_none(),
                "retired station remains: {retired}"
            );
        }
        let mut ids = std::collections::BTreeSet::new();
        for station in curated_stations() {
            assert!(ids.insert(station.id), "duplicate station ID");
            assert!(
                station_tcp_target(station.id).is_some(),
                "missing target: {}",
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
            assert!(!station.genre.is_empty());
            assert!(station.playlist_url.starts_with("https://"));
            assert!(station.publisher_reference_url.starts_with("https://"));
            assert!(station.third_party_player_support);
            assert_eq!(station.rights_reviewed_date.len(), 10);
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
            Some(("stream.rcs.revma.com", 443))
        );
        assert_eq!(
            station_by_id("kunc").unwrap().playlist_url,
            "https://ais-sa1.streamon.fm/7890_96k.mp3"
        );
        assert_eq!(station_tcp_target("unknown"), None);
        assert_eq!(station_tcp_target("https://attacker.test"), None);
    }
}
