#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
#![allow(unused_imports)]
use tracing::{debug, error, info};

mod active_session_pipe;
#[cfg(feature = "enable_client")]
mod client;
mod core;
mod lock_request;
mod msg;
mod recoverable_session_pipe;
#[cfg(feature = "enable_server")]
mod redirect;
mod registry;
#[cfg(feature = "enable_server")]
mod server;
mod session;
mod test;

use async_trait::async_trait;
use bytes::Bytes;
use fxhash::FxHashMap;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex as StdMutex;
use std::sync::RwLock as StdRwLock;
use std::sync::Weak;
use std::{collections::hash_map::Entry, collections::BTreeMap, sync::Arc};
use std::{
    net::{IpAddr, Ipv6Addr},
    str::FromStr,
};
use tokio::sync::mpsc;
use tokio::sync::{Mutex, RwLock};

use super::chain::*;
use super::chain::*;
use super::comms::*;
use super::conf::*;
use super::crypto::AteHash;
use super::error::*;
use super::event::*;
#[cfg(feature = "enable_server")]
use super::flow::*;
use super::session::*;
use super::transaction::*;
use super::trust::*;
use crate::dio::*;
use crate::engine::TaskEngine;
#[cfg(feature = "enable_server")]
use crate::flow::basic::*;
use crate::mesh::msg::*;
use crate::meta::*;
use crate::pipe::*;

#[cfg(feature = "enable_client")]
pub(crate) use crate::mesh::client::MeshClient;

pub(crate) use session::MeshSession;

pub use crate::mesh::core::MeshHashTable;
pub use self::core::BackupMode;
pub use self::core::RecoveryMode;
pub use self::msg::FatalTerminate;
pub use crate::loader::Loader;
pub use crate::mesh::registry::ChainGuard;
pub use crate::mesh::registry::Registry;
#[cfg(feature = "enable_server")]
pub use crate::mesh::server::MeshRoot;

fn create_prepare<'a, 'b>(cfg_mesh: &'b ConfMesh) -> Vec<MeshAddress> {
    let mut hash_table = BTreeMap::new();
    for addr in cfg_mesh.roots.iter() {
        hash_table.insert(addr.hash(), addr.clone());
    }

    #[allow(unused_mut)]
    let mut listen_root_addresses = Vec::new();

    #[cfg(feature = "enable_server")]
    if let Some(addr) = &cfg_mesh.force_listen {
        listen_root_addresses.push(addr.clone());
    }

    #[cfg(feature = "enable_dns")]
    if listen_root_addresses.len() <= 0 && cfg_mesh.force_client_only == false {
        let local_ips = pnet::datalink::interfaces()
            .iter()
            .flat_map(|i| i.ips.iter())
            .map(|i| i.ip())
            .collect::<Vec<_>>();
        for local_ip in local_ips.iter() {
            for root in cfg_mesh.roots.iter() {
                if root.host == *local_ip {
                    listen_root_addresses.push(root.clone());
                }
            }
        }
    }

    listen_root_addresses
}

#[cfg(feature = "enable_server")]
pub async fn create_persistent_centralized_server(
    cfg_ate: &ConfAte,
    cfg_mesh: &ConfMesh,
) -> Result<Arc<MeshRoot>, CommsError> {
    let ret = create_server(cfg_mesh).await?;
    ret.add_route(super::flow::all_persistent_and_centralized().await, cfg_ate)
        .await?;
    Ok(ret)
}

#[cfg(feature = "enable_server")]
pub async fn create_persistent_distributed_server(
    cfg_ate: &ConfAte,
    cfg_mesh: &ConfMesh,
) -> Result<Arc<MeshRoot>, CommsError> {
    let ret = create_server(cfg_mesh).await?;
    ret.add_route(super::flow::all_persistent_and_distributed().await, cfg_ate)
        .await?;
    Ok(ret)
}

#[cfg(feature = "enable_server")]
pub async fn create_ethereal_centralized_server(
    cfg_ate: &ConfAte,
    cfg_mesh: &ConfMesh,
) -> Result<Arc<MeshRoot>, CommsError> {
    let ret = create_server(cfg_mesh).await?;
    ret.add_route(super::flow::all_ethereal_centralized().await, cfg_ate)
        .await?;
    Ok(ret)
}

#[cfg(feature = "enable_server")]
pub async fn create_ethereal_distributed_server(
    cfg_ate: &ConfAte,
    cfg_mesh: &ConfMesh,
) -> Result<Arc<MeshRoot>, CommsError> {
    let ret = create_server(cfg_mesh).await?;
    ret.add_route(super::flow::all_ethereal_distributed().await, cfg_ate)
        .await?;
    Ok(ret)
}

#[cfg(feature = "enable_server")]
pub async fn create_server(cfg_mesh: &ConfMesh) -> Result<Arc<MeshRoot>, CommsError> {
    let listen_root_addresses = create_prepare(cfg_mesh);
    let ret = MeshRoot::new(&cfg_mesh, listen_root_addresses).await?;

    Ok(ret)
}

#[cfg(feature = "enable_client")]
pub fn create_client(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh, temporal: bool) -> Arc<MeshClient> {
    let client_id = NodeId::generate_client_id();
    MeshClient::new(&cfg_ate, &cfg_mesh, client_id, temporal)
}

#[cfg(feature = "enable_client")]
pub fn create_persistent_client(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshClient> {
    let client_id = NodeId::generate_client_id();
    MeshClient::new(&cfg_ate, &cfg_mesh, client_id, false)
}

#[cfg(feature = "enable_client")]
pub fn create_temporal_client(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshClient> {
    let client_id = NodeId::generate_client_id();
    MeshClient::new(&cfg_ate, &cfg_mesh, client_id, true)
}

pub static GLOBAL_CERTIFICATES: Lazy<StdRwLock<Vec<AteHash>>> =
    Lazy::new(|| StdRwLock::new(Vec::new()));

pub(crate) static GLOBAL_COMM_FACTORY: Lazy<
    Mutex<
        Option<
            Arc<
                dyn Fn(
                        MeshConnectAddr,
                    )
                        -> Pin<Box<dyn Future<Output = Option<Stream>> + Send + Sync + 'static>>
                    + Send
                    + Sync
                    + 'static,
            >,
        >,
    >,
> = Lazy::new(|| Mutex::new(None));

pub fn add_global_certificate(cert: &AteHash) {
    GLOBAL_CERTIFICATES.write().unwrap().push(cert.clone());
}

pub async fn set_comm_factory(
    funct: impl Fn(MeshConnectAddr) -> Pin<Box<dyn Future<Output = Option<Stream>> + Send + Sync + 'static>>
        + Send
        + Sync
        + 'static,
) {
    GLOBAL_COMM_FACTORY.lock().await.replace(Arc::new(funct));
}
