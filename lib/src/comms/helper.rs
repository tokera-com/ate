#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Serialize, de::DeserializeOwned};
#[cfg(feature = "enable_full")]
use tokio::{net::{TcpStream}};
use bytes::Bytes;
#[allow(unused_imports)]
use tokio::io::{self};
use tokio::io::Error as TError;
use tokio::io::ErrorKind;
use tokio::sync::broadcast;
use async_trait::async_trait;
use std::net::SocketAddr;
use parking_lot::Mutex as StdMutex;
use tokio::select;

use crate::comms::*;
use crate::spec::*;
use crate::crypto::*;
use crate::error::*;
use crate::comms::NodeId;

use super::Packet;
use super::PacketData;
use super::PacketWithContext;
use super::StreamRx;
use super::Metrics;
use super::Throttle;

#[cfg(feature="enable_dns")]
type MeshConnectAddr = SocketAddr;
#[cfg(not(feature="enable_dns"))]
type MeshConnectAddr = crate::conf::MeshAddress;

#[async_trait]
pub(crate) trait InboxProcessor<M, C>
where Self: Send + Sync,
      M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
      C: Send + Sync,
{
    async fn process(&mut self, pck: PacketWithContext<M, C>) -> Result<(), CommsError>;

    async fn shutdown(&mut self, addr: MeshConnectAddr);
}

#[cfg(feature = "enable_full")]
pub(super) fn setup_tcp_stream(stream: &TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?;
    Ok(())
}

#[allow(unused_variables)]
pub(super) async fn process_inbox<M, C>(
    mut rx: StreamRx,
    mut inbox: Box<dyn InboxProcessor<M, C>>,
    metrics: Arc<StdMutex<Metrics>>,
    throttle: Arc<StdMutex<Throttle>>,
    id: NodeId,
    peer_id: NodeId,
    sock_addr: MeshConnectAddr,
    context: Arc<C>,
    wire_format: SerializationFormat,
    wire_encryption: Option<EncryptKey>,
    mut exit: broadcast::Receiver<()>
) -> Result<(), CommsError>
where M: Send + Sync + Serialize + DeserializeOwned + Clone + Default,
      C: Send + Sync,
{
    let ret = async {
        // Throttling variables
        let throttle_interval = chrono::Duration::milliseconds(50);
        let mut last_throttle = chrono::offset::Utc::now();
        let mut current_received = 0u64;
        let mut current_sent = 0u64;
        let mut hickup_count = 0u32;

        // Main read loop
        loop
        {
            // Read the next request
            let mut total_read = 0u64;
            let buf = async
            {
                // If the throttle has triggered
                let now = chrono::offset::Utc::now();
                let delta = now - last_throttle;
                if delta > throttle_interval {
                    last_throttle = now;
                    
                    // Compute the deltas
                    let (mut delta_received, mut delta_sent) = {
                        let metrics = metrics.lock();
                        let delta_received = metrics.received - current_received;
                        let delta_sent = metrics.sent - current_sent;
                        current_received = metrics.received;
                        current_sent = metrics.sent;
                        (delta_received as i64, delta_sent as i64)
                    };

                    // Normalize the delta based on the time that passed
                    delta_received *= 1000i64;
                    delta_sent *= 1000i64;
                    delta_received /= delta.num_milliseconds();
                    delta_sent /= delta.num_milliseconds();

                    // We throttle the connection based off the current metrics and a calculated wait time
                    let wait_time = {
                        let throttle = throttle.lock();
                        let wait1 = throttle.download_per_second
                            .map(|limit| limit as i64)
                            .filter(|limit| delta_sent.gt(limit))
                            .map(|limit| chrono::Duration::milliseconds(((delta_sent-limit) * 1000i64) / limit));
                        let wait2 = throttle.upload_per_second
                            .map(|limit| limit as i64)
                            .filter(|limit| delta_received.gt(limit))
                            .map(|limit| chrono::Duration::milliseconds(((delta_received-limit) * 1000i64) / limit));

                        // Whichever is the longer wait is the one we shall do
                        match (wait1, wait2) {
                            (Some(a), Some(b)) if a >= b => Some(a),
                            (Some(_), Some(b)) => Some(b),
                            (Some(a), None) => Some(a),
                            (None, Some(b)) => Some(b),
                            (None, None) => None
                        }
                    };

                    // We wait outside the throttle lock otherwise we will break things
                    if let Some(wait_time) = wait_time {
                        if let Ok(wait_time) = wait_time.to_std() {
                            trace!("trottle wait: {}ms", wait_time.as_millis());
                            tokio::time::sleep(wait_time).await;
                        }
                    }
                }

                match wire_encryption {
                    Some(key) => {
                        // Read the initialization vector
                        let iv_bytes = rx.read_8bit().await?;
                        total_read += 1u64;
                        match iv_bytes.len() {
                            0 => Err(TError::new(ErrorKind::BrokenPipe, "iv_bytes-len is zero")),
                            l => {
                                total_read += l as u64;
                                let iv = InitializationVector::from(iv_bytes);

                                // Read the cipher text and decrypt it
                                let cipher_bytes = rx.read_32bit().await?;
                                total_read += 4u64;
                                match cipher_bytes.len() {
                                    0 => Err(TError::new(ErrorKind::BrokenPipe, "cipher_bytes-len is zero")),
                                    l => {
                                        total_read += l as u64;
                                        Ok(key.decrypt(&iv, &cipher_bytes))
                                    }
                                }
                            }
                        }
                    },
                    None => {
                        // Read the next message
                        let buf = rx.read_32bit().await?;
                        total_read += 4u64;
                        match buf.len() {
                            0 => Err(TError::new(ErrorKind::BrokenPipe, "buf-len is zero")),
                            l => {
                                total_read += l as u64;
                                Ok(buf)
                            }
                        }
                    }
                }
            };
            let buf = {
                select! {
                    _ = exit.recv() => {
                        debug!("received exit broadcast - {}", sock_addr);
                        break;
                    },
                    a = buf => a
                }
            }?;

            // Update the metrics with all this received data
            {
                let mut metrics = metrics.lock();
                metrics.received += total_read;
                metrics.requests += 1u64;
            }
                
            // Deserialize it
            let msg: M = wire_format.deserialize(&buf[..])?;
            let pck = Packet {
                msg,
            };
            
            // Process it
            let pck = PacketWithContext {
                data: PacketData {
                    bytes: Bytes::from(buf),
                    wire_format,
                },
                context: Arc::clone(&context),
                packet: pck,
                id,
                peer_id,
            };

            // Its time to process the packet
            let rcv = inbox.process(pck);
            match rcv.await {
                Ok(a) => {
                    if hickup_count > 0 {
                        debug!("inbox-recovered: recovered from hickups {}", hickup_count);    
                    }
                    hickup_count = 0;
                    a
                },
                Err(CommsError(CommsErrorKind::Disconnected, _)) => { break; }
                Err(CommsError(CommsErrorKind::IO(err), _)) if err.kind() == std::io::ErrorKind::BrokenPipe => {
                    if rx.protocol().is_web_socket() && hickup_count < 10 {
                        hickup_count += 1;
                        continue;
                    }
                    debug!("inbox-debug: {}", err);
                    break;
                }
                Err(CommsError(CommsErrorKind::IO(err), _)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                    debug!("inbox-debug: {}", err);
                    break;
                }
                Err(CommsError(CommsErrorKind::IO(err), _)) if err.kind() == std::io::ErrorKind::ConnectionAborted => {
                    warn!("inbox-err: {}", err);
                    break;
                }
                Err(CommsError(CommsErrorKind::IO(err), _)) if err.kind() == std::io::ErrorKind::ConnectionReset => {
                    warn!("inbox-err: {}", err);
                    break;
                }
                Err(CommsError(CommsErrorKind::ReadOnly, _)) => { continue; }
                Err(CommsError(CommsErrorKind::NotYetSubscribed, _)) => {
                    error!("inbox-err: {}", CommsErrorKind::NotYetSubscribed);
                    break;
                }
                Err(CommsError(CommsErrorKind::CertificateTooWeak(needed, actual), _)) => {
                    error!("inbox-err: {}", CommsErrorKind::CertificateTooWeak(needed, actual));
                    break;
                }
                Err(CommsError(CommsErrorKind::MissingCertificate, _)) => {
                    error!("inbox-err: {}", CommsErrorKind::MissingCertificate);
                    break;
                }
                Err(CommsError(CommsErrorKind::ServerCertificateValidation, _)) => {
                    error!("inbox-err: {}", CommsErrorKind::ServerCertificateValidation);
                    break;
                }
                Err(CommsError(CommsErrorKind::ServerEncryptionWeak, _)) => {
                    error!("inbox-err: {}", CommsErrorKind::ServerEncryptionWeak);
                    break;
                }
                Err(CommsError(CommsErrorKind::FatalError(err), _)) => {
                    error!("inbox-err: {}", err);
                    break;
                }
                Err(CommsError(CommsErrorKind::SendError(err), _)) => {
                    warn!("inbox-err: {}", err);
                    break;
                }
                Err(CommsError(CommsErrorKind::ValidationError(ValidationErrorKind::Many(errs)), _)) => {
                    for err in errs.iter() {
                        trace!("val-err: {}", err);
                    }

                    #[cfg(debug_assertions)]
                    warn!("inbox-debug: {} validation errors", errs.len());
                    #[cfg(not(debug_assertions))]
                    debug!("inbox-debug: {} validation errors", errs.len());
                    continue;
                }
                Err(CommsError(CommsErrorKind::ValidationError(err), _)) => {
                    #[cfg(debug_assertions)]
                    warn!("inbox-debug: validation error - {}", err);
                    #[cfg(not(debug_assertions))]
                    debug!("inbox-debug: validation error - {}", err);
                    continue;
                }
                Err(err) => {
                    warn!("inbox-error: {}", err.to_string());
                    continue;
                }
            }
        }
        Ok(())
    }.await;

    inbox.shutdown(sock_addr).await;
    ret
}