//! This file contains some fairly meaningless glue code for integrating with libp2p.

use std::net::{IpAddr, SocketAddr};

use libp2p::{multiaddr::Protocol, Multiaddr};

use crate::P2P;

#[derive(Clone)]
pub struct SpaceTimeProtocolName(String);

impl SpaceTimeProtocolName {
	pub fn new(p2p: &P2P) -> Self {
		SpaceTimeProtocolName(format!("/{}/spacetime/1.0.0", p2p.app_name()))
	}
}

impl AsRef<str> for SpaceTimeProtocolName {
	fn as_ref(&self) -> &str {
		&self.0
	}
}

// TODO: Turn these into From/Into impls on a wrapper type

pub(crate) fn quic_multiaddr_to_socketaddr(m: Multiaddr) -> Result<SocketAddr, String> {
	let mut addr_parts = m.iter();

	let addr = match addr_parts.next() {
		Some(Protocol::Ip4(addr)) => IpAddr::V4(addr),
		Some(Protocol::Ip6(addr)) => IpAddr::V6(addr),
		Some(proto) => {
			return Err(format!(
				"Invalid multiaddr. Segment 1 found protocol 'Ip4' or 'Ip6' but found  '{proto}'"
			))
		}
		None => return Err("Invalid multiaddr. Segment 1 missing".to_string()),
	};

	let port = match addr_parts.next() {
		Some(Protocol::Udp(port)) => port,
		Some(proto) => {
			return Err(format!(
				"Invalid multiaddr. Segment 2 expected protocol 'Udp' but found  '{proto}'"
			))
		}
		None => return Err("Invalid multiaddr. Segment 2 missing".to_string()),
	};

	Ok(SocketAddr::new(addr, port))
}

#[must_use]
pub(crate) fn socketaddr_to_quic_multiaddr(m: &SocketAddr) -> Multiaddr {
	let mut addr = Multiaddr::empty();
	match m {
		SocketAddr::V4(ip) => addr.push(Protocol::Ip4(*ip.ip())),
		SocketAddr::V6(ip) => addr.push(Protocol::Ip6(*ip.ip())),
	}
	addr.push(Protocol::Udp(m.port()));
	addr.push(Protocol::QuicV1);
	addr
}