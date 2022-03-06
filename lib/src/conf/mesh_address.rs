use serde::{Deserialize, Serialize};
#[cfg(feature = "enable_dns")]
use std::net::IpAddr;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::crypto::AteHash;

/// Represents a target node within a mesh
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct MeshAddress {
    #[cfg(feature = "enable_dns")]
    pub host: IpAddr,
    #[cfg(not(feature = "enable_dns"))]
    pub host: String,
    pub port: u16,
}

#[cfg(feature = "enable_dns")]
pub type MeshConnectAddr = std::net::SocketAddr;
#[cfg(not(feature = "enable_dns"))]
pub type MeshConnectAddr = crate::conf::MeshAddress;

impl MeshAddress {
    #[cfg(feature = "enable_dns")]
    #[allow(dead_code)]
    pub fn new(ip: IpAddr, port: u16) -> MeshAddress {
        MeshAddress { host: ip, port }
    }

    #[cfg(not(feature = "enable_dns"))]
    #[allow(dead_code)]
    pub fn new(domain: &str, port: u16) -> MeshAddress {
        MeshAddress {
            host: domain.to_string(),
            port,
        }
    }

    pub fn hash(&self) -> AteHash {
        #[cfg(feature = "enable_dns")]
        match self.host {
            IpAddr::V4(host) => AteHash::from_bytes_twice(&host.octets(), &self.port.to_be_bytes()),
            IpAddr::V6(host) => AteHash::from_bytes_twice(&host.octets(), &self.port.to_be_bytes()),
        }
        #[cfg(not(feature = "enable_dns"))]
        AteHash::from_bytes_twice(self.host.as_bytes(), &self.port.to_be_bytes())
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl std::fmt::Display for MeshAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}
