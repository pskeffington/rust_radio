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
    pub nominal_kbps: u16,
    /// Publisher-declared codec hint; runtime validation records decoded format.
    pub format: &'static str,
}

/// Curated broadcaster links checked against publisher pages 2026-10-26.
/// Inclusion is not proof a stream is online, licensed for redistribution,
/// or supported by the local decoder at runtime.
/// The broadcaster's conditions allow personal listening, not rebroadcasting.
pub const CURATED_STATIONS: [InternetStation; 25] = [
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
        nominal_kbps: 0,
        format: "AAC",
    },
    InternetStation {
        id: "groovesalad",
        name: "Groove Salad",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Ambient / downtempo",
        category: "CHILL",
        playlist_url: "https://somafm.com/m3u/groovesalad.m3u",
        publisher_reference_url: "https://somafm.com/groovesalad/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "secretagent",
        name: "Secret Agent",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Spy jazz / lounge / downtempo",
        category: "CHILL",
        playlist_url: "https://somafm.com/m3u/secretagent.m3u",
        publisher_reference_url: "https://somafm.com/secretagent/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "lush",
        name: "Lush",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Mellow vocals / electronic",
        category: "CHILL",
        playlist_url: "https://somafm.com/m3u/lush.m3u",
        publisher_reference_url: "https://somafm.com/lush/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "dronezone",
        name: "Drone Zone",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Atmospheric / ambient",
        category: "AMBIENT",
        playlist_url: "https://somafm.com/m3u/dronezone.m3u",
        publisher_reference_url: "https://somafm.com/dronezone/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "deepspaceone",
        name: "Deep Space One",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Space / experimental",
        category: "AMBIENT",
        playlist_url: "https://somafm.com/m3u/deepspaceone.m3u",
        publisher_reference_url: "https://somafm.com/deepspaceone/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "spacestation",
        name: "Space Station Soma",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Space / midtempo electronica",
        category: "AMBIENT",
        playlist_url: "https://somafm.com/m3u/spacestation.m3u",
        publisher_reference_url: "https://somafm.com/spacestation/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "beatblender",
        name: "Beat Blender",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Deep house / downtempo",
        category: "ELECTRONIC",
        playlist_url: "https://somafm.com/m3u/beatblender.m3u",
        publisher_reference_url: "https://somafm.com/beatblender/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "defcon",
        name: "DEF CON Radio",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Electronic / hacker radio",
        category: "ELECTRONIC",
        playlist_url: "https://somafm.com/m3u/defcon.m3u",
        publisher_reference_url: "https://somafm.com/defcon/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "u80s",
        name: "Underground 80s",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Synthpop / new wave",
        category: "RETRO",
        playlist_url: "https://somafm.com/m3u/u80s.m3u",
        publisher_reference_url: "https://somafm.com/u80s/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "indiepop",
        name: "Indie Pop Rocks!",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Indie pop / alternative",
        category: "INDIE",
        playlist_url: "https://somafm.com/m3u/indiepop.m3u",
        publisher_reference_url: "https://somafm.com/indiepop/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "folkfwd",
        name: "Folk Forward",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Indie folk / acoustic",
        category: "FOLK",
        playlist_url: "https://somafm.com/m3u/folkfwd.m3u",
        publisher_reference_url: "https://somafm.com/folkfwd/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "metal",
        name: "Metal Detector",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Metal / hard rock",
        category: "ROCK",
        playlist_url: "https://somafm.com/m3u/metal.m3u",
        publisher_reference_url: "https://somafm.com/metal/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "bootliquor",
        name: "Boot Liquor",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Americana / roots / country",
        category: "ROOTS",
        playlist_url: "https://somafm.com/m3u/bootliquor.m3u",
        publisher_reference_url: "https://somafm.com/bootliquor/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "sonicuniverse",
        name: "Sonic Universe",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Modern jazz / avant-garde",
        category: "JAZZ",
        playlist_url: "https://somafm.com/m3u/sonicuniverse.m3u",
        publisher_reference_url: "https://somafm.com/sonicuniverse/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "bossa",
        name: "Bossa Beyond",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Bossa nova / Brazilian",
        category: "WORLD",
        playlist_url: "https://somafm.com/m3u/bossa.m3u",
        publisher_reference_url: "https://somafm.com/bossa/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "reggae",
        name: "Heavyweight Reggae",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Reggae / ska / rocksteady",
        category: "WORLD",
        playlist_url: "https://somafm.com/m3u/reggae.m3u",
        publisher_reference_url: "https://somafm.com/reggae/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "thistle",
        name: "ThistleRadio",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Celtic / traditional",
        category: "WORLD",
        playlist_url: "https://somafm.com/m3u/thistle.m3u",
        publisher_reference_url: "https://somafm.com/thistle/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "fluid",
        name: "Fluid",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Instrumental hip-hop / future soul",
        category: "HIPHOP",
        playlist_url: "https://somafm.com/m3u/fluid.m3u",
        publisher_reference_url: "https://somafm.com/fluid/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "cliqhop",
        name: "cliqhop idm",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "IDM / experimental electronic",
        category: "ELECTRONIC",
        playlist_url: "https://somafm.com/m3u/cliqhop.m3u",
        publisher_reference_url: "https://somafm.com/cliqhop/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "thetrip",
        name: "The Trip",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Progressive house / trance",
        category: "DANCE",
        playlist_url: "https://somafm.com/m3u/thetrip.m3u",
        publisher_reference_url: "https://somafm.com/thetrip/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "poptron",
        name: "PopTron",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Electropop / indie dance",
        category: "POP",
        playlist_url: "https://somafm.com/m3u/poptron.m3u",
        publisher_reference_url: "https://somafm.com/poptron/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "leftcoast70s",
        name: "Left Coast 70s",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "1970s album rock / soft rock",
        category: "RETRO",
        playlist_url: "https://somafm.com/m3u/seventies.m3u",
        publisher_reference_url: "https://somafm.com/seventies/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "illstreet",
        name: "Illinois Street Lounge",
        broadcaster: "SomaFM",
        language: "en",
        country: "US",
        genre: "Exotica / vintage lounge",
        category: "LOUNGE",
        playlist_url: "https://somafm.com/m3u/illstreet.m3u",
        publisher_reference_url: "https://somafm.com/illstreet/directstreamlinks.html",
        nominal_kbps: 128,
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
        assert_eq!(curated_stations().len(), 25);
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
                "LOUNGE"
            ]
            .contains(&station.category));
            assert!(!station.genre.is_empty());
            assert!(station.playlist_url.starts_with("https://"));
            assert!(station.publisher_reference_url.starts_with("https://"));
            if station.id == "skylab" {
                assert_eq!(
                    station.playlist_url,
                    "https://stream.rcs.revma.com/7vs4sr22w3bwv.pls"
                );
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
            } else {
                assert!(station.playlist_url.starts_with("https://somafm.com/m3u/"));
                assert!(station.playlist_url.ends_with(".m3u"));
                assert_eq!(station.format, "MP3");
                assert_eq!(station.nominal_kbps, 128);
            }
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
        assert_eq!(station_tcp_target("unknown"), None);
        assert_eq!(station_tcp_target("https://attacker.test"), None);
    }
}
