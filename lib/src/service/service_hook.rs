use tracing::Instrument;
#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use fxhash::FxHashSet;
use error_chain::bail;
use async_trait::async_trait;
use std::sync::{Arc, Weak};
use bytes::Bytes;

use crate::{crypto::AteHash, error::*, event::*, meta::{CoreMetadata}, spec::MessageFormat};
use crate::chain::*;
use crate::session::*;
use crate::meta::*;
use crate::header::*;
use crate::prelude::TransactionScope;
use crate::prelude::DioMut;
use crate::dio::row::RowData;
use crate::dio::row::RowHeader;
use crate::engine::TaskEngine;

use super::*;

pub struct ServiceHook
{
    pub session: AteSession,
    pub scope: TransactionScope,
    handler: Arc<dyn ServiceInvoker>,
    chain: Weak<Chain>,
}

impl ServiceHook
{
    pub(crate) fn new(chain: &Arc<Chain>, session: AteSession, handler: &Arc<dyn ServiceInvoker>) -> ServiceHook {
        ServiceHook {
            chain: Arc::downgrade(chain),
            session: session.clone(),
            handler: Arc::clone(handler),
            scope: TransactionScope::None,
        }
    }
}

#[async_trait]
impl Service
for ServiceHook
{
    fn filter(&self, evt: &EventData) -> bool {
        if let Some(t) = evt.meta.get_type_name() {
            return t.type_name == self.handler.request_type_name();
        }
        false
    }

    async fn notify(&self, key: PrimaryKey) -> Result<(), InvokeError>
    {
        // Get a reference to the chain
        let chain = match self.chain.upgrade() {
            Some(a) => a,
            None => {
                bail!(InvokeErrorKind::Aborted);
            }
        };

        // Start a span for this notify
        let span = span!(Level::DEBUG, "service", id=chain.node_id.to_short_string().as_str());
        let _span = span.enter();

        // Build the data access layer
        let dio = chain.dio_trans(&self.session, self.scope).await;
        dio.auto_cancel();

        // Lock the data row
        if dio.try_lock(key).await? == false {
            debug!("service call skipped - someone else locked it");
            return Ok(())
        }

        // Load the object and lock it (to prevent others processing it)
        let mut evt = dio.load_raw(&key).await?;
        
        // Convert the data using the encryption and decryption routines
        dio.data_as_overlay(&mut evt)?;
        let req = match evt.data_bytes {
            Some(a) => a,
            None => { bail!(InvokeErrorKind::NoData); }
        };

        // Invoke the callback in the service
        let ret = self.handler.invoke(req).await?;

        // Commit the results - If an error occurs cancel everything and delete the command
        if let Err(_) = &ret {
            dio.cancel();
        }

        // We delete the row under a concurrent task to prevent deadlocks
        dio.delete(&key).await?;

        // Process the results
        let reply_ret =   match ret {
            Ok(res) => {
                debug!("service [{}] ok", self.handler.request_type_name());
                trace!("sending {}", self.handler.response_type_name());
                self.send_reply(&dio, key, res, self.handler.response_type_name())
            },
            Err(err) => {
                debug!("service [{}] error", self.handler.request_type_name());
                trace!("sending {}", self.handler.error_type_name());
                self.send_reply(&dio, key, err, self.handler.error_type_name())
            }
        };

        // We commit the transactions that holds the reply message under a concurrent
        // thread to prevent deadlocks
        TaskEngine::spawn(async move {
            let span = span!(Level::DEBUG, "service", id=chain.node_id.to_short_string().as_str());
            let ret = dio.commit()
                .instrument(span)
                .await;
            if let Err(err) = ret {
                debug!("notify-err - {}", err);
            }
        });

        // If the reply failed to send then return that error - otherwise success!
        reply_ret?;
        Ok(())
    }
}

impl ServiceHook
{
    fn send_reply(&self, dio: &Arc<DioMut>, req: PrimaryKey, res: Bytes, res_type: String) -> Result<(), InvokeError>
    {
        let key = PrimaryKey::generate();
        let format = self.handler.data_format();
        let data = res;
        let data_hash = AteHash::from_bytes(&data[..]);

        let mut auth = MetaAuthorization::default();
        auth.write = WriteOption::Nobody;

        let parent = Some(MetaParent{ vec: 
            MetaCollection {
                parent_id: req,
                collection_id: fastrand::u64(..),
            }
        });

        let mut extra_meta = Vec::new();
        extra_meta.push(CoreMetadata::Type(MetaType {
            type_name: res_type.clone()
        }));
        extra_meta.push(CoreMetadata::Reply(req));

        let mut state = dio.state.lock();
        state.dirty_header(RowHeader {
            key,
            parent: parent.clone(),
            auth: auth.clone(),
        });
        state.dirty_row(RowData {
            key,
            type_name: res_type,
            format: MessageFormat {
                data: format,
                meta: dio.default_format().meta,
            },
            data_hash,
            data,
            collections: FxHashSet::default(),
            created: 0,
            updated: 0,
            extra_meta,
            parent: parent.clone(),
            auth,
        });
        Ok(())
    }
}