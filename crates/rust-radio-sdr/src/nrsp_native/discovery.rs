//! Bounded, read-only local receiver discovery on the native browser port.
use super::*;
use std::collections::BTreeSet;
use std::sync::{atomic::AtomicUsize, Mutex};

pub const MAX_SCAN_HOSTS: usize = 512;
const SCAN_DURATION: Duration = Duration::from_secs(10);
const PORT_DURATION: Duration = Duration::from_millis(200);
const HELLO_DURATION: Duration = Duration::from_secs(3);
const WORKERS: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredReceiver {
    pub endpoint: String,
    pub status: Status,
}

/// Inspect at most two local /24-sized private IPv4 neighborhoods. Larger
/// subnets are capped to the interface's own /24; network/broadcast/self are
/// excluded. An optional explicit private endpoint is checked first.
pub fn discover(
    cancel: &AtomicBool,
    explicit: Option<&str>,
) -> Result<Vec<DiscoveredReceiver>, Error> {
    let interfaces = if_addrs::get_if_addrs().map_err(transport)?;
    let addresses = interfaces
        .into_iter()
        .filter(|interface| interface.is_oper_up() && !interface.is_p2p())
        .filter_map(|interface| match interface.addr {
            if_addrs::IfAddr::V4(address) if address.ip.is_private() => {
                Some((address.ip, address.netmask))
            }
            _ => None,
        });
    let mut targets = Vec::new();
    if let Some(value) = explicit.filter(|value| !value.is_empty()) {
        targets.push(Endpoint::parse(value)?.0);
    }
    targets.extend(candidates(addresses));
    let mut seen = BTreeSet::new();
    targets.retain(|target| seen.insert(*target));
    targets.truncate(MAX_SCAN_HOSTS);
    scan(&targets, cancel)
}

fn candidates(interfaces: impl IntoIterator<Item = (Ipv4Addr, Ipv4Addr)>) -> Vec<SocketAddr> {
    let interfaces: Vec<_> = interfaces.into_iter().collect();
    let local: BTreeSet<_> = interfaces.iter().map(|(ip, _)| u32::from(*ip)).collect();
    let mut result = BTreeSet::new();
    let mut neighborhoods = BTreeSet::new();
    for (ip, mask) in interfaces {
        if !ip.is_private() {
            continue;
        }
        let address = u32::from(ip);
        let mask = u32::from(mask);
        // Ignore invalid/noncontiguous netmasks and host-only interfaces.
        if (!mask).wrapping_add(1) & !mask != 0 || mask == u32::MAX {
            continue;
        }
        let limited_mask = mask | 0xffffff00;
        let network = address & limited_mask;
        let broadcast = network | !limited_mask;
        if neighborhoods.contains(&(network, limited_mask)) {
            continue;
        }
        if neighborhoods.len() >= 2 {
            break;
        }
        neighborhoods.insert((network, limited_mask));
        for host in network.saturating_add(1)..broadcast {
            if !local.contains(&host) {
                result.insert(SocketAddr::from((Ipv4Addr::from(host), 9001)));
            }
            if result.len() >= MAX_SCAN_HOSTS {
                return result.into_iter().collect();
            }
        }
    }
    result.into_iter().collect()
}

fn scan(targets: &[SocketAddr], cancel: &AtomicBool) -> Result<Vec<DiscoveredReceiver>, Error> {
    let deadline = Instant::now() + SCAN_DURATION;
    check(deadline, cancel)?;
    let next = AtomicUsize::new(0);
    let found = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..WORKERS.min(targets.len()) {
            scope.spawn(|| {
                while !cancel.load(Ordering::Acquire) && Instant::now() < deadline {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(address) = targets.get(index) else {
                        break;
                    };
                    let port_deadline = deadline
                        .saturating_duration_since(Instant::now())
                        .min(PORT_DURATION);
                    if port_deadline.is_zero()
                        || TcpStream::connect_timeout(address, port_deadline).is_err()
                    {
                        continue;
                    }
                    let host_deadline = (Instant::now() + HELLO_DURATION).min(deadline);
                    if let Ok(client) =
                        Client::connect_until(Endpoint(*address), cancel, host_deadline)
                    {
                        found
                            .lock()
                            .expect("discovery mutex poisoned")
                            .push(DiscoveredReceiver {
                                endpoint: address.to_string(),
                                status: client.status().clone(),
                            });
                    }
                }
            });
        }
    });
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Cancelled);
    }
    let mut result = found.into_inner().expect("discovery mutex poisoned");
    result.sort_by(|a, b| a.endpoint.cmp(&b.endpoint));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn neighborhood_is_private_bounded_and_excludes_network_broadcast_self() {
        let targets = candidates([
            (
                Ipv4Addr::new(192, 168, 2, 10),
                Ipv4Addr::new(255, 255, 255, 0),
            ),
            (Ipv4Addr::new(8, 8, 8, 8), Ipv4Addr::UNSPECIFIED),
        ]);
        assert_eq!(targets.len(), 253);
        for last in [0, 10, 255] {
            assert!(!targets.contains(&SocketAddr::from(([192, 168, 2, last], 9001))));
        }
        let targets = candidates([(Ipv4Addr::new(10, 1, 2, 3), Ipv4Addr::new(255, 0, 0, 0))]);
        assert_eq!(targets.len(), 253);
        assert!(targets
            .iter()
            .all(|address| address.ip().to_string().starts_with("10.1.2.")));
        assert!(matches!(
            scan(&[], &AtomicBool::new(true)),
            Err(Error::Cancelled)
        ));
    }
    #[test]
    fn discovery_validates_native_hello_without_radio_commands() {
        let (endpoint, server) = super::super::tests::fixture(false, |mut socket, _listener| {
            use std::io::Read;
            assert_eq!(socket.get_mut().read(&mut [0]).unwrap(), 0);
        });
        let result = scan(&[endpoint.0], &AtomicBool::new(false)).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status.center_frequency_hz, 99_100_000);
        server.join().unwrap();
    }
}
