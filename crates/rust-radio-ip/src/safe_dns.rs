//! DNS resolution policy for radio HTTP requests.
//!
//! Curated stations and public direct streams must not resolve to local or
//! special-use addresses. The resolver returns the checked addresses to
//! reqwest, so it connects to the same set that passed the policy check rather
//! than doing a second, potentially rebound lookup.

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

type ResolverError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub(crate) struct SafeResolver {
    allow_private: bool,
}

impl SafeResolver {
    pub(crate) fn public_only() -> Arc<Self> {
        Arc::new(Self {
            allow_private: false,
        })
    }

    pub(crate) fn explicit_private_source() -> Arc<Self> {
        Arc::new(Self {
            allow_private: true,
        })
    }
}

impl Resolve for SafeResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_owned();
        let allow_private = self.allow_private;
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
                .any(|address| !is_allowed_address(address.ip(), allow_private))
            {
                return Err(Box::new(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "radio host resolved to a non-public address",
                )) as ResolverError);
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

pub(crate) fn is_allowed_address(address: IpAddr, allow_private: bool) -> bool {
    is_public_unicast(address) || (allow_private && is_private_unicast(address))
}

fn is_private_unicast(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback(),
        IpAddr::V6(ip) => ip.is_loopback() || ip.segments()[0] & 0xfe00 == 0xfc00,
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
            "192.0.2.1",
            "198.18.0.1",
            "203.0.113.1",
            "224.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
            "2001:20::1",
            "2002:0808:0808::1",
            "3fff::1",
        ] {
            let address = IpAddr::from_str(address).unwrap();
            assert!(!is_public_unicast(address), "{address}");
        }
        assert!(is_allowed_address("192.168.1.5".parse().unwrap(), true));
        assert!(is_allowed_address("::1".parse().unwrap(), true));
        assert!(!is_allowed_address("169.254.1.2".parse().unwrap(), true));
        assert!(!is_allowed_address("224.0.0.1".parse().unwrap(), true));
        assert!(!is_allowed_address(
            "255.255.255.255".parse().unwrap(),
            true
        ));
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
    fn public_resolver_rejects_localhost_and_private_mode_allows_explicit_lan_hosts() {
        let name = Name::from_str("localhost").unwrap();
        let resolver = SafeResolver::public_only();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(resolver.resolve(name));
        assert!(result.is_err());

        let name = Name::from_str("localhost").unwrap();
        let resolver = SafeResolver::explicit_private_source();
        assert!(runtime.block_on(resolver.resolve(name)).is_ok());
    }
}
