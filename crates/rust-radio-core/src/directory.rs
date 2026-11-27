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

/// Small starter set of direct GBH radio streams. The publisher lists GBH 89.7
/// and CRB 99.5 as available through third-party radio apps. Inclusion is not
/// proof a feed is online or licensed for commercial redistribution.
pub const CURATED_STATIONS: [InternetStation; 2] = [
    InternetStation {
        id: "gbh897",
        name: "GBH 89.7 News Radio (Boston)",
        broadcaster: "GBH",
        language: "en",
        country: "US",
        genre: "Public radio news / talk",
        category: "NEWS",
        playlist_url: "https://wgbh-live.streamguys1.com/wgbh",
        publisher_reference_url: "https://www.wgbh.org/help/how-to-access-live-radio-streams",
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
        format: "Streaming audio (verify at runtime)",
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
        let Some((host, port)) = CURATED_STATIONS
            .first()
            .and_then(|station| station_tcp_target(station.id))
        else {
            return IpReachability::Unreachable;
        };
        let Ok(addresses) = (host, port).to_socket_addrs() else {
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
            station_by_id("gbh897").map(|s| s.name),
            Some("GBH 89.7 News Radio (Boston)")
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
        assert_eq!(curated_stations().len(), 2);
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
                "NEWS",
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
            assert_eq!(station.broadcaster, "GBH");
            assert_eq!(
                station.publisher_reference_url,
                "https://www.wgbh.org/help/how-to-access-live-radio-streams"
            );
            if station.id == "gbh897" {
                assert_eq!(
                    station.playlist_url,
                    "https://wgbh-live.streamguys1.com/wgbh"
                );
            } else {
                assert_eq!(station.id, "wcrb");
                assert_eq!(
                    station.playlist_url,
                    "https://wgbh-live.streamguys1.com/classical-hi/"
                );
                assert_eq!(station.format, "AAC");
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
            station_tcp_target("gbh897"),
            Some(("wgbh-live.streamguys1.com", 443))
        );
        assert_eq!(station_tcp_target("unknown"), None);
        assert_eq!(station_tcp_target("https://attacker.test"), None);
    }
}
