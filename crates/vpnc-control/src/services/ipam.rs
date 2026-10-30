//! IP address management for tunnel addresses.

use anyhow::{bail, Context};
use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Allocates addresses from a CIDR, skipping network/broadcast (IPv4) and already used ones.
#[derive(Debug, Default)]
pub struct IpAllocator {
    used_v4: BTreeSet<Ipv4Addr>,
    used_v6: BTreeSet<Ipv6Addr>,
}

impl IpAllocator {
    /// Creates an empty allocator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks an address as already used (e.g. loaded from DB).
    pub fn mark_used(&mut self, addr: IpAddr) {
        match addr {
            IpAddr::V4(a) => {
                self.used_v4.insert(a);
            }
            IpAddr::V6(a) => {
                self.used_v6.insert(a);
            }
        }
    }

    /// Releases an address back to the pool.
    pub fn release(&mut self, addr: IpAddr) {
        match addr {
            IpAddr::V4(a) => {
                self.used_v4.remove(&a);
            }
            IpAddr::V6(a) => {
                self.used_v6.remove(&a);
            }
        }
    }

    /// Allocates the next free IPv4 from `cidr`.
    pub fn allocate_v4(&mut self, cidr: &str) -> anyhow::Result<Ipv4Addr> {
        let net: Ipv4Net = cidr.parse().context("parse ipv4 cidr")?;
        let hosts: Vec<Ipv4Addr> = net.hosts().collect();
        if hosts.is_empty() {
            bail!("cidr too small for host allocation: {cidr}");
        }
        for host in hosts {
            if !self.used_v4.contains(&host) {
                self.used_v4.insert(host);
                return Ok(host);
            }
        }
        bail!("ipv4 pool exhausted: {cidr}");
    }

    /// Allocates the next free IPv6 from `cidr` (iterates first 65536 hosts max).
    pub fn allocate_v6(&mut self, cidr: &str) -> anyhow::Result<Ipv6Addr> {
        let net: Ipv6Net = cidr.parse().context("parse ipv6 cidr")?;
        for host in net.hosts().take(65_536) {
            if !self.used_v6.contains(&host) {
                self.used_v6.insert(host);
                return Ok(host);
            }
        }
        bail!("ipv6 pool exhausted: {cidr}");
    }

    /// Parses a generic CIDR string into [`IpNet`].
    pub fn parse_net(cidr: &str) -> anyhow::Result<IpNet> {
        Ok(cidr.parse()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocates_and_releases_v4() {
        let mut ipam = IpAllocator::new();
        let a = ipam.allocate_v4("10.0.0.0/30").unwrap();
        let b = ipam.allocate_v4("10.0.0.0/30").unwrap();
        assert_ne!(a, b);
        ipam.release(IpAddr::V4(a));
        let c = ipam.allocate_v4("10.0.0.0/30").unwrap();
        assert_eq!(c, a);
    }

    #[test]
    fn exhausts_small_pool() {
        let mut ipam = IpAllocator::new();
        // /30 has 2 hosts
        let _ = ipam.allocate_v4("192.168.1.0/30").unwrap();
        let _ = ipam.allocate_v4("192.168.1.0/30").unwrap();
        let err = ipam.allocate_v4("192.168.1.0/30").unwrap_err();
        assert!(err.to_string().contains("exhausted"));
    }

    #[test]
    fn allocates_v6() {
        let mut ipam = IpAllocator::new();
        let a = ipam.allocate_v6("fd00::/127").unwrap();
        assert_ne!(a, std::net::Ipv6Addr::UNSPECIFIED);
    }
}
