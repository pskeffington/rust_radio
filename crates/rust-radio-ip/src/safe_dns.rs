//! DNS resolution policy for radio HTTP requests.
//!
//! Curated stations and public direct streams must not resolve to local or
//! special-use addresses. The resolver returns the checked addresses to
//! reqwest, so it connects to the same set that passed the policy check rather
//! than doing a second, potentially rebound lookup.

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::fmt;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

type ResolverError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub(crate) struct NonPublicAddressResolution;

impl fmt::Display for NonPublicAddressResolution {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("radio host resolved to a non-public address")
    }
}

impl std::error::Error for NonPublicAddressResolution {}

#[derive(Debug)]
pub(crate) struct SafeResolver;

impl SafeResolver {
    pub(crate) fn public_only() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl Resolve for SafeResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
                .await
                .map_err(|error| Box::new(error) as ResolverError)?
                .collect();
            if addresses.is_empty() {
                return Err(Box::new(io::Error::new(
                    io::ErrorKind::NotFound,
                    "radio host resolved to no addresses",
                )) as ResolverError);
            }
            if addresses
                .iter()
                .any(|address| !is_public_unicast(address.ip()))
            {
                return Err(Box::new(NonPublicAddressResolution) as ResolverError);
            }
            Ok(Box::new(addresses.into_iter()) as Addrs)
        })
    }
}

pub(crate) fn is_public_unicast(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => is_public_ipv6(ip),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip == Ipv4Addr::BROADCAST
        || a == 0
        || (a == 100 && (64..=127).contains(&b)) // shared address space (RFC 6598)
        || (a == 192 && b == 0 && c == 0) // protocol assignments
        || (a == 192 && b == 0 && c == 2) // documentation
        || (a == 192 && b == 88 && c == 99) // 6to4 relay anycast
        || (a == 198 && (b == 18 || b == 19)) // benchmarking
        || (a == 198 && b == 51 && c == 100) // documentation
        || (a == 203 && b == 0 && c == 113) // documentation
        || a >= 240) // reserved
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    let global_unicast = segments[0] & 0xe000 == 0x2000;
    let special_2001 = segments[0] == 0x2001 && segments[1] <= 0x01ff;
    let documentation_2001 = segments[0] == 0x2001 && segments[1] == 0x0db8;
    let documentation_3fff = segments[0] == 0x3fff && segments[1] & 0xf000 == 0;
    let nat64_translation = segments[0] == 0x0064 && segments[1] == 0xff9b;
    global_unicast
        && !ip.is_unspecified()
        && !ip.is_loopback()
        && !ip.is_multicast()
        && segments[0] & 0xfe00 != 0xfc00 // unique-local
        && segments[0] & 0xffc0 != 0xfe80 // unicast link-local
        && !special_2001
        && !documentation_2001
        && !documentation_3fff
        && !nat64_translation
        && segments[0] != 0x2002 // 6to4 can encapsulate non-public IPv4 targets
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn public_address_filter_rejects_private_and_special_use_ranges() {
        for address in [
            "0.1.2.3",
            "10.1.2.3",
            "100.64.1.2",
            "127.0.0.1",
            "169.254.1.2",
            "192.0.0.9",
            "192.0.2.1",
            "192.0.0.170",
            "192.88.99.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "::ffff:192.168.1.1",
            "fc00::1",
            "fe80::1",
            "64:ff9b::c0a8:101",
            "64:ff9b:1::1",
            "100::1",
            "100:0:0:1::1",
            "2001:2::1",
            "2001:db8::1",
            "2001:20::1",
            "2002:0808:0808::1",
            "3fff::1",
            "5f00::1",
        ] {
            let address = IpAddr::from_str(address).unwrap();
            assert!(!is_public_unicast(address), "{address}");
        }
        for address in [
            "8.8.8.8",
            "1.1.1.1",
            "2606:4700:4700::1111",
            "2001:4860:4860::8888",
        ] {
            let address = IpAddr::from_str(address).unwrap();
            assert!(is_public_unicast(address), "{address}");
        }
    }

    #[test]
    fn public_resolver_rejects_localhost_even_for_an_explicit_private_stream() {
        let name = Name::from_str("localhost").unwrap();
        let resolver = SafeResolver::public_only();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(resolver.resolve(name));
        assert!(result.is_err());
    }
}
