#[allow(unused_imports)]
use log::{info, warn, debug};
use rand::seq::SliceRandom;
use fxhash::FxHashMap;
use tokio::sync::mpsc;
use std::{marker::PhantomData};
#[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
use tokio::sync::broadcast;
#[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
use std::sync::Arc;
use serde::{Serialize, de::DeserializeOwned};

use crate::error::*;
use crate::prelude::SerializationFormat;

use super::conf::Upstream;
use super::Packet;
use super::PacketData;
use super::BroadcastPacketData;
use super::PacketWithContext;

#[derive(Debug)]
pub(crate) enum TxDirection
{
    #[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
    Downcast(Arc<broadcast::Sender<BroadcastPacketData>>),
    UpcastOne(Upstream),
    UpcastMany(FxHashMap<u64, Upstream>)
}

#[derive(Debug)]
pub(crate) struct NodeTx<C>
where C: Send + Sync
{
    pub hello_path: String,
    pub direction: TxDirection,
    pub wire_format: SerializationFormat,
    pub _marker: PhantomData<C>,
}

#[derive(Debug)]
pub(crate) struct NodeRx<M, C>
where M: Send + Sync + Serialize + DeserializeOwned + Clone,
      C: Send + Sync
{
    pub rx: mpsc::Receiver<PacketWithContext<M, C>>,
    pub _marker: PhantomData<C>,
}

#[allow(dead_code)]
impl<C> NodeTx<C>
where C: Send + Sync + Default + 'static
{
    pub(crate) async fn send_packet(&self, pck: BroadcastPacketData) -> Result<(), CommsError> {
        match &self.direction {
            #[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
            TxDirection::Downcast(a) => {
                a.send(pck)?;
            },
            TxDirection::UpcastOne(a) => {
                a.outbox.send(pck.data).await?;
            },
            TxDirection::UpcastMany(a) => {
                let upcasts = a.values().filter(|u| u.outbox.is_closed() == false).collect::<Vec<_>>();
                let upcast = upcasts.choose(&mut rand::thread_rng()).unwrap();
                upcast.outbox.send(pck.data).await?;
            }
        };
        Ok(())
    }

    pub(crate) fn get_unicast_sender(&self) -> Option<mpsc::Sender<PacketData>>
    {
        match &self.direction {
            #[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
            TxDirection::Downcast(_) => {
                None
            },
            TxDirection::UpcastOne(a) => {
                Some(a.outbox.clone())
            },
            TxDirection::UpcastMany(a) => {
                let upcasts = a.values().filter(|u| u.outbox.is_closed() == false).collect::<Vec<_>>();
                let upcast = upcasts.choose(&mut rand::thread_rng()).unwrap();
                Some(upcast.outbox.clone())
            }
        }
    }

    pub(crate) async fn send<M>(&self, msg: M, broadcast_group: Option<u64>) -> Result<(), CommsError>
    where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default
    {
        self.send_packet(BroadcastPacketData {
            group: broadcast_group,
            data: Packet::from(msg).to_packet_data(self.wire_format)?
        }).await
    }

    pub(crate) async fn on_disconnect(&self) -> Result<(), CommsError> {
        match &self.direction {
            #[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
            TxDirection::Downcast(_) => {
                return Err(CommsError::ShouldBlock);
            },
            TxDirection::UpcastOne(a) => {
                a.outbox.closed().await;
            },
            TxDirection::UpcastMany(a) => {
                for u in a.values() {
                    u.outbox.closed().await;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn is_closed(&self) -> bool {
        match &self.direction {
            #[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
            TxDirection::Downcast(_) => {
                false
            },
            TxDirection::UpcastOne(a) => {
                a.outbox.is_closed()
            },
            TxDirection::UpcastMany(a) => {
                a.values().any(|u| u.outbox.is_closed() == false) == false
            }
        }
    }
}

#[allow(dead_code)]
impl<M, C> NodeRx<M, C>
where C: Send + Sync + Default + 'static,
      M: Send + Sync + Serialize + DeserializeOwned + Clone + Default
{
    pub async fn recv(&mut self) -> Option<PacketWithContext<M, C>>
    {
        self.rx.recv().await
    }
}

impl<C> Drop
for NodeTx<C>
where C: Send + Sync
{
    fn drop(&mut self)
    {
        #[cfg(feature = "enable_super_verbose")]
        debug!("drop");
    }
}