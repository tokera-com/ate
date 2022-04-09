use std::sync::Mutex as StdMutex;
use std::time::Duration;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use derivative::*;

use crate::error::*;

use crate::comms::Metrics;
use crate::comms::NodeId;
use crate::comms::Throttle;
use crate::transaction::*;

use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::sync::broadcast;
use tokio::sync::RwLock;

use crate::conf::ConfAte;
use crate::conf::MeshAddress;
use crate::mesh::BackupMode;
use crate::meta::*;
use crate::multi::*;
use crate::pipe::*;
use crate::prelude::PrimaryKey;
use crate::redo::RedoLog;
use crate::single::*;
use crate::spec::*;
use crate::time::TimeKeeper;
use crate::transaction::TransactionScope;
use crate::trust::ChainHeader;
use crate::trust::ChainKey;

use super::*;

/// Represents the main API to access a specific chain-of-trust
///
/// This object must stay within scope for the duration of its
/// use which has been optimized for infrequent initialization as
/// creating this object will reload the entire chain's metadata
/// into memory.
///
/// The actual data of the chain is stored locally on disk thus
/// huge chains can be stored here however very random access on
/// large chains will result in random access IO on the disk.
///
/// Chains also allow subscribe/publish models to be applied to
/// particular vectors (see the examples for details)
///
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct Chain {
    pub(crate) key: ChainKey,
    #[allow(dead_code)]
    pub(crate) node_id: NodeId,
    pub(crate) cfg_ate: ConfAte,
    pub(crate) remote: Option<url::Url>,
    pub(crate) remote_addr: Option<MeshAddress>,
    pub(crate) default_format: MessageFormat,
    #[derivative(Debug = "ignore")]
    pub(crate) inside_sync: Arc<StdRwLock<ChainProtectedSync>>,
    pub(crate) inside_async: Arc<RwLock<ChainProtectedAsync>>,
    #[derivative(Debug = "ignore")]
    pub(crate) pipe: Arc<Box<dyn EventPipe>>,
    pub(crate) time: Arc<TimeKeeper>,
    pub(crate) exit: broadcast::Sender<()>,
    pub(crate) decache: broadcast::Sender<Vec<PrimaryKey>>,
    pub(crate) metrics: Arc<StdMutex<Metrics>>,
    pub(crate) throttle: Arc<StdMutex<Throttle>>,
}

impl<'a> Chain {
    pub(crate) fn proxy(&mut self, mut proxy: Box<dyn EventPipe>) {
        proxy.set_next(Arc::clone(&self.pipe));
        let proxy = Arc::new(proxy);
        let _ = std::mem::replace(&mut self.pipe, proxy);
    }

    pub fn key(&'a self) -> &'a ChainKey {
        &self.key
    }

    pub fn remote(&'a self) -> Option<&'a url::Url> {
        self.remote.as_ref()
    }

    pub fn remote_addr(&'a self) -> Option<&'a MeshAddress> {
        self.remote_addr.as_ref()
    }

    pub async fn single(&'a self) -> ChainSingleUser<'a> {
        ChainSingleUser::new(self).await
    }

    pub async fn multi(&'a self) -> ChainMultiUser {
        ChainMultiUser::new(self).await
    }

    pub async fn name(&'a self) -> String {
        self.single().await.name()
    }

    pub fn default_format(&'a self) -> MessageFormat {
        self.default_format.clone()
    }

    pub async fn count(&'a self) -> usize {
        self.inside_async.read().await.chain.redo.count()
    }

    pub async fn flush(&'a self) -> Result<(), tokio::io::Error> {
        Ok(self.inside_async.write().await.chain.flush().await?)
    }

    pub async fn sync(&'a self) -> Result<(), CommitError> {
        let timeout = Duration::from_secs(30);
        self.sync_ext(timeout).await
    }

    pub async fn sync_ext(&'a self, timeout: Duration) -> Result<(), CommitError> {
        // Create the transaction
        let trans = Transaction {
            scope: TransactionScope::Full,
            transmit: true,
            events: Vec::new(),
            timeout,
            conversation: None,
        };

        // Feed the transaction into the chain
        let pipe = self.pipe.clone();
        pipe.feed(ChainWork { trans }).await?;

        // Success!
        Ok(())
    }

    pub(crate) async fn get_pending_uploads(&self) -> Vec<MetaDelayedUpload> {
        let guard = self.inside_async.read().await;
        guard.chain.timeline.pointers.get_pending_uploads()
    }

    pub fn metrics(&'a self) -> &'a Arc<StdMutex<Metrics>> {
        &self.metrics
    }

    pub fn throttle(&'a self) -> &'a Arc<StdMutex<Throttle>> {
        &self.throttle
    }

    pub async fn shutdown(&self) -> Result<(), CompactError> {
        let include_active_files = match self.cfg_ate.backup_mode {
            BackupMode::None => {
                return Ok(());
            }
            BackupMode::Restore => {
                return Ok(());
            }
            BackupMode::Rotating => false,
            BackupMode::Full => true,
        };

        let mut single = self.single().await;
        let we_are_the_one = if single.inside_async.is_shutdown == false {
            single.inside_async.is_shutdown = true;

            single
                .inside_async
                .chain
                .redo
                .backup(include_active_files)?
                .await?;
            true
        } else {
            false
        };
        drop(single);

        if we_are_the_one {
            #[cfg(feature = "enable_local_fs")]
            if self.cfg_ate.log_path.is_some() && self.cfg_ate.compact_cleanup {
                self.compact().await?;
            }
        }

        Ok(())
    }
}

impl Drop for Chain {
    fn drop(&mut self) {
        trace!("drop {}", self.key.to_string());
        let _ = self.exit.send(());
    }
}

impl RedoLog {
    pub(crate) fn read_chain_header(&self) -> Result<ChainHeader, SerializationError> {
        let header_bytes = self.header(u32::MAX);
        Ok(if header_bytes.len() > 0 {
            SerializationFormat::Json.deserialize(&header_bytes[..])?
        } else {
            ChainHeader::default()
        })
    }
}
