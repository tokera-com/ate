#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
#![allow(unused_imports)]
use log::{error, info, debug};

mod msg;
mod core;
mod root;
mod client;
mod session;
mod registry;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::{net::{IpAddr, Ipv6Addr}, str::FromStr};
use tokio::sync::{RwLock, Mutex};
use std::sync::Mutex as StdMutex;
use std::{collections::BTreeMap, sync::Arc, collections::hash_map::Entry};
use tokio::sync::mpsc;
use std::sync::mpsc as smpsc;
use fxhash::FxHashMap;
use crate::{flow::basic::OpenStaticBuilder, meta::Metadata, pipe::EventPipe};
use bytes::Bytes;
use std::sync::Weak;

use super::flow::*;
use super::crypto::Hash;
use super::event::*;
use super::comms::*;
use super::trust::*;
use super::chain::*;
use super::error::*;
use super::chain::*;
use super::conf::*;
use super::transaction::*;
use super::session::*;
use crate::mesh::msg::*;
use crate::dio::DaoVec;
use crate::dio::Dao;
use crate::dio::DaoObj;

use crate::mesh::client::MeshClient;
use crate::mesh::root::MeshRoot;

pub(crate) use super::mesh::session::MeshSession;
pub use crate::mesh::registry::Registry;

fn create_prepare<'a, 'b>(cfg_mesh: &'b ConfMesh) -> (Vec<MeshAddress>, Option<&'b ConfCluster>) {
    let mut hash_table = BTreeMap::new();
    for addr in cfg_mesh.clusters.iter().flat_map(|c| c.roots.iter()) {
        hash_table.insert(addr.hash(), addr.clone());
    }

    let local_ips = pnet::datalink::interfaces()
        .iter()
        .flat_map(|i| i.ips.iter())
        .map(|i| i.ip())
        .collect::<Vec<_>>();

    let mut listen_cluster = cfg_mesh.clusters.iter().next();
    let mut listen_root_addresses = Vec::new();
    
    if let Some(addr) = &cfg_mesh.force_listen {
        listen_root_addresses.push(addr.clone());
        listen_cluster = cfg_mesh.clusters.iter().filter(|c| c.roots.contains(addr)).next();
    } else if cfg_mesh.force_client_only == false {
        for local_ip in local_ips.iter() {
            for cfg_cluster in cfg_mesh.clusters.iter() {
                for root in cfg_cluster.roots.iter() {
                    if root.ip == *local_ip {
                        listen_cluster = Some(cfg_cluster);
                        listen_root_addresses.push(root.clone());
                    }
                }
            }
        }
    }

    (listen_root_addresses, listen_cluster)
}

pub async fn create_persistent_server(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshRoot<OpenStaticBuilder>>
{
    create_server(cfg_ate, cfg_mesh, super::flow::all_persistent(cfg_ate)).await
}

pub async fn create_ethereal_server(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshRoot<OpenStaticBuilder>>
{
    create_server(cfg_ate, cfg_mesh, super::flow::all_ethereal(cfg_ate)).await
}

pub async fn create_server<F>(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh, open_flow: Box<F>) -> Arc<MeshRoot<F>>
where F: OpenFlow + 'static
{
    
    let (listen_root_addresses, listen_cluster) = create_prepare(cfg_mesh);
    MeshRoot::new(
        &cfg_ate,
        &cfg_mesh,
        listen_cluster,
        listen_root_addresses,
        open_flow).await
}

pub async fn create_client(cfg_ate: &ConfAte, cfg_mesh: &ConfMesh) -> Arc<MeshClient>
{
    MeshClient::new(&cfg_ate, &cfg_mesh).await
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct TestData {
    pub data: u128,
    pub inner: DaoVec<String>,
}

#[tokio::main]
#[test]
async fn test_mesh()
{
    env_logger::init();

    let cfg_ate = ConfAte::default();

    let mut mesh_roots = Vec::new();
    let mut cfg_mesh = {
        let mut cluster1 = ConfCluster::default();
        for n in 5100..5105 {
            cluster1.roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
        }

        let mut cluster2 = ConfCluster::default();
        for n in 6100..6105 {
            cluster2.roots.push(MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n));
        }  

        let mut cfg_mesh = ConfMesh::default();
        cfg_mesh.clusters.push(cluster1);
        cfg_mesh.clusters.push(cluster2);

        let mut mesh_root_joins = Vec::new();
        for n in 5100..5105 {
            let addr = MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n);
            let cfg_ate = cfg_ate.clone();
            let mut cfg_mesh = cfg_mesh.clone();
            cfg_mesh.force_listen = Some(addr.clone());

            let join = tokio::spawn(async move {
                create_server(&cfg_ate, &cfg_mesh, super::flow::all_ethereal(&cfg_ate)).await
            });
            mesh_root_joins.push((addr, join));
        }
        for n in 6100..6105 {
            let addr = MeshAddress::new(IpAddr::from_str("127.0.0.1").unwrap(), n);
            let cfg_ate = cfg_ate.clone();
            let mut cfg_mesh = cfg_mesh.clone();
            cfg_mesh.force_listen = Some(addr.clone());

            let join = tokio::spawn(async move {
                create_server(&cfg_ate, &cfg_mesh, super::flow::all_ethereal(&cfg_ate)).await
            });
            mesh_root_joins.push((addr, join));
        }
        
        for (addr, join) in mesh_root_joins {
            debug!("creating server on {:?}", addr);
            mesh_roots.push(join.await);
        }
        cfg_mesh
    };

    debug!("create the mesh and connect to it with client 1");
    let client_a = create_client(&cfg_ate, &cfg_mesh).await;
    let chain_a = client_a.open(&url::Url::parse("tcp://127.0.0.1/test-chain").unwrap()).await.unwrap();
    let session_a = Session::default();
    
    let dao_key1;
    let dao_key2;
    {
        let mut bus;
        let task;

        {
            let mut dio = chain_a.dio_ext(&session_a, Scope::Full).await;
            let dao2: Dao<TestData> = dio.store(TestData::default()).unwrap();
            dao_key2 = dao2.key().clone();

            bus = dao2.bus(&chain_a, dao2.inner);
            task = bus.recv(&session_a);
            dio.commit().await.unwrap();
        }

        {
            cfg_mesh.force_listen = None;
            cfg_mesh.force_client_only = true;
            let client_b = create_client(&cfg_ate, &cfg_mesh).await;

            let chain_b = client_b.open(&url::Url::parse("tcp://127.0.0.1/test-chain").unwrap()).await.unwrap();
            let session_b = Session::default();
            {
                debug!("start a DIO session for client B");
                let mut dio = chain_b.dio_ext(&session_b, Scope::Full).await;

                debug!("store data object 1");
                dao_key1 = dio.store(TestData::default()).unwrap().key().clone();

                debug!("load data object 2");
                let mut dao2: Dao<TestData> = dio.load(&dao_key2).await.expect("An earlier saved object should have loaded");
                
                debug!("add to new sub objects to the vector");
                dao2.push(&mut dio, dao2.inner, "test_string1".to_string()).unwrap();
                dao2.push(&mut dio, dao2.inner, "test_string2".to_string()).unwrap();

                debug!("commit the DIO");
                dio.commit().await.unwrap();
            }
        }

        debug!("sync to disk");
        chain_a.sync().await.unwrap();

        debug!("wait for an event on the BUS");
        let task_ret = task.await.expect("Should have received the result on the BUS");
        assert_eq!(*task_ret, "test_string1".to_string());

        {
            debug!("new DIO session for client A");
            let mut dio = chain_a.dio_ext(&session_a, Scope::Full).await;

            debug!("processing the next event in the BUS (and lock_for_delete it)");
            let task = bus.process(&mut dio);
            let mut task_ret = task.await.expect("Should have received the result on the BUS for the second time");
            assert_eq!(*task_ret, "test_string2".to_string());

            // Committing the DIO
            task_ret.commit(&mut dio).unwrap();

            debug!("loading data object 1");
            dio.load::<TestData>(&dao_key1).await.expect("The data did not not get replicated to other clients in realtime");
            
            debug!("committing the DIO");
            dio.commit().await.unwrap();
        }
    }

    {
        cfg_mesh.force_listen = None;
        cfg_mesh.force_client_only = true;
        let client = create_client(&cfg_ate, &cfg_mesh).await;

        debug!("reconnecting the client");
        let chain = client.open(&url::Url::parse("tcp://127.0.0.1/test-chain").unwrap()).await.unwrap();
        let session = Session::default();
        {
            debug!("loading data object 1");
            let mut dio = chain.dio_ext(&session, Scope::Full).await;
            dio.load::<TestData>(&dao_key1).await.expect("The data did not survive between new sessions");
        }
    }

    debug!("shutting down");
    //std::process::exit(0);
}