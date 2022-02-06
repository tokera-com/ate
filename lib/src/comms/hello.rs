use error_chain::bail;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::crypto::KeySize;
use crate::error::*;
use crate::spec::*;
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HelloMetadata {
    pub client_id: NodeId,
    pub server_id: NodeId,
    pub path: String,
    pub encryption: Option<KeySize>,
    pub wire_format: SerializationFormat,
}

/// Version of the stream protocol used to talk to Tokera services
#[repr(u16)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum StreamProtocolVersion
{
    V1 = 1,
    V2 = 2,
}

impl StreamProtocolVersion
{
    pub fn min(&self, other: StreamProtocolVersion) -> StreamProtocolVersion {
        let first = *self as u16;
        let second = other as u16;
        let min = first.min(second);

        if first == min {
            *self
        } else {
            other
        }
    }

    pub fn create(&self) -> Box<dyn super::protocol::api::MessageProtocolApi + 'static>
    {
        match self {
            StreamProtocolVersion::V1 => {
                Box::new(super::protocol::v1::MessageProtocol::default())
            }
            StreamProtocolVersion::V2 => {
                Box::new(super::protocol::v2::MessageProtocol::default())
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SenderHello {
    pub id: NodeId,
    pub path: String,
    pub domain: String,
    pub key_size: Option<KeySize>,
    #[serde(default = "default_stream_protocol_version")]
    pub version: StreamProtocolVersion,
}

fn default_stream_protocol_version() -> StreamProtocolVersion {
    StreamProtocolVersion::V1
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ReceiverHello {
    pub id: NodeId,
    pub encryption: Option<KeySize>,
    pub wire_format: SerializationFormat,
    #[serde(default = "default_stream_protocol_version")]
    pub version: StreamProtocolVersion,
}

#[cfg(feature = "enable_client")]
pub async fn mesh_hello_exchange_sender(
    stream_rx: &mut StreamRx,
    stream_tx: &mut StreamTx,
    client_id: NodeId,
    hello_path: String,
    domain: String,
    key_size: Option<KeySize>,
) -> Result<HelloMetadata, CommsError> {
    // Send over the hello message and wait for a response
    trace!("client sending hello");
    let hello_client = SenderHello {
        id: client_id,
        path: hello_path.clone(),
        domain,
        key_size,
        version: StreamProtocolVersion::V2,
    };
    let hello_client_bytes = serde_json::to_vec(&hello_client)?;
    stream_tx
        .write_with_fixed_16bit_header(&hello_client_bytes[..], false)
        .await?;

    // Read the hello message from the other side
    let hello_server_bytes = stream_rx.read_with_fixed_16bit_header().await?;
    trace!("client received hello from server");
    trace!("{}", String::from_utf8_lossy(&hello_server_bytes[..]));
    let hello_server: ReceiverHello = serde_json::from_slice(&hello_server_bytes[..])?;

    // Validate the encryption is strong enough
    if let Some(needed_size) = &key_size {
        match &hello_server.encryption {
            None => {
                bail!(CommsErrorKind::ServerEncryptionWeak);
            }
            Some(a) if *a < *needed_size => {
                bail!(CommsErrorKind::ServerEncryptionWeak);
            }
            _ => {}
        }
    }

    // Switch to the correct protocol version
    let version = hello_server.version.min(hello_client.version);
    stream_rx.change_protocol_version(version.create());
    stream_tx.change_protocol_version(version.create());

    // Upgrade the key_size if the server is bigger
    trace!(
        "client encryption={}",
        match &hello_server.encryption {
            Some(a) => a.as_str(),
            None => "none",
        }
    );
    trace!("client wire_format={}", hello_server.wire_format);

    Ok(HelloMetadata {
        client_id,
        server_id: hello_server.id,
        path: hello_path,
        encryption: hello_server.encryption,
        wire_format: hello_server.wire_format,
    })
}

#[cfg(feature = "enable_server")]
pub async fn mesh_hello_exchange_receiver(
    stream_rx: &mut StreamRx,
    stream_tx: &mut StreamTx,
    server_id: NodeId,
    key_size: Option<KeySize>,
    wire_format: SerializationFormat,
) -> Result<HelloMetadata, CommsError> {
    // Read the hello message from the other side
    let hello_client_bytes = stream_rx.read_with_fixed_16bit_header().await?;
    trace!("server received hello from client");
    //trace!("server received hello from client: {}", String::from_utf8_lossy(&hello_client_bytes[..]));
    let hello_client: SenderHello = serde_json::from_slice(&hello_client_bytes[..])?;

    // Upgrade the key_size if the client is bigger
    let encryption = mesh_hello_upgrade_key(key_size, hello_client.key_size);

    // Send over the hello message and wait for a response
    trace!("server sending hello (wire_format={})", wire_format);
    let hello_server = ReceiverHello {
        id: server_id,
        encryption,
        wire_format,
        version: StreamProtocolVersion::V2,
    };
    let hello_server_bytes = serde_json::to_vec(&hello_server)?;
    stream_tx
        .write_with_fixed_16bit_header(&hello_server_bytes[..], false)
        .await?;

    // Switch to the correct protocol version
    let version = hello_server.version.min(hello_client.version);
    stream_rx.change_protocol_version(version.create());
    stream_tx.change_protocol_version(version.create());

    Ok(HelloMetadata {
        client_id: hello_client.id,
        server_id,
        path: hello_client.path,
        encryption,
        wire_format,
    })
}

#[cfg(feature = "enable_server")]
fn mesh_hello_upgrade_key(key1: Option<KeySize>, key2: Option<KeySize>) -> Option<KeySize> {
    // If both don't want encryption then who are we to argue about that?
    if key1.is_none() && key2.is_none() {
        return None;
    }

    // Wanting encryption takes priority over not wanting encyption
    let key1 = match key1 {
        Some(a) => a,
        None => {
            trace!("upgrading to {}bit shared secret", key2.unwrap());
            return key2;
        }
    };
    let key2 = match key2 {
        Some(a) => a,
        None => {
            trace!("upgrading to {}bit shared secret", key1);
            return Some(key1);
        }
    };

    // Upgrade the key_size if the client is bigger
    if key2 > key1 {
        trace!("upgrading to {}bit shared secret", key2);
        return Some(key2);
    }
    if key1 > key2 {
        trace!("upgrading to {}bit shared secret", key2);
        return Some(key1);
    }

    // They are identical
    return Some(key1);
}
