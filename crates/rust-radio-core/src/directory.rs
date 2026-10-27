//! Curated, receive-only internet radio fallback.
//!
//! Source selection is pure and testable. Actual SDR discovery and station
//! playback are independently gated; TCP reachability is not stream validation.

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// A first-party, publisher-documented stream directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InternetStation {
    pub id: &'static str,
    pub name: &'static str,
    pub genre: &'static str,
    /// Short, operator-visible catalog grouping; not a broadcast band.
    pub category: &'static str,
    pub playlist_url: &'static str,
    pub publisher_reference_url: &'static str,
    pub nominal_kbps: u16,
    pub format: &'static str,
}

/// Curated broadcaster links checked against publisher pages 2026-10-26.
/// Inclusion is not proof a stream is online, licensed for redistribution,
/// or supported by the local decoder at runtime.
/// The broadcaster's conditions allow personal listening, not rebroadcasting.
pub const CURATED_STATIONS: [InternetStation; 10] = [
    InternetStation {
        id: "skylab",
        name: "Skylab Radio (UK)",
        genre: "Chillout / lounge / ambient",
        category: "CHILL",
        playlist_url: "https://uksoutha.streaming.broadcast.radio:29690/skylab-radio-limited",
        publisher_reference_url: "https://www.skylabradio.com/listen",
        nominal_kbps: 0,
        format: "Streaming audio (verify at runtime)",
    },
    InternetStation {
        id: "groovesalad",
        name: "Groove Salad",
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
        genre: "Synthpop / new wave",
        category: "RETRO",
        playlist_url: "https://somafm.com/m3u/u80s.m3u",
        publisher_reference_url: "https://somafm.com/u80s/directstreamlinks.html",
        nominal_kbps: 128,
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
        let Ok(addresses) = ("somafm.com", 443).to_socket_addrs() else {
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
    let (host, port) = match id {
        "skylab" => ("uksoutha.streaming.broadcast.radio", 29690),
        "groovesalad" | "secretagent" | "lush" | "dronezone" | "deepspaceone" | "spacestation"
        | "beatblender" | "defcon" | "u80s" => ("somafm.com", 443),
        _ => return None,
    };
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
        assert_eq!(curated_stations().len(), 10);
        let mut ids = std::collections::BTreeSet::new();
        for station in curated_stations() {
            assert!(ids.insert(station.id), "duplicate station ID");
            assert!(["CHILL", "AMBIENT", "ELECTRONIC", "RETRO"].contains(&station.category));
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
            } else {
                assert!(station.playlist_url.starts_with("https://somafm.com/m3u/"));
                assert!(station.playlist_url.ends_with(".m3u"));
                assert_eq!(station.format, "MP3");
                assert_eq!(station.nominal_kbps, 128);
            }
            assert!(!station.name.is_empty());
        }
    }
}
