use bytes::Bytes;

use crate::crypto::{DoubleHash, Hash};

use super::header::*;
use super::meta::*;
use super::error::*;

extern crate rmp_serde as rmps;

/// Represents the raw bytes that can describe what the event is
#[derive(Debug, Clone)]
pub struct EventHeaderRaw
{
    pub meta_hash: super::crypto::Hash,
    pub meta_bytes: Bytes,
    pub data_hash: Option<super::crypto::Hash>,
    pub event_hash: super::crypto::Hash,
}

impl EventHeaderRaw
{
    pub(crate) fn new(meta_hash: super::crypto::Hash, meta_bytes: Bytes, data_hash: Option<super::crypto::Hash>) -> EventHeaderRaw
    {
        EventHeaderRaw {
            event_hash: match &data_hash {
                Some(data_hash) => DoubleHash::from_hashes(&meta_hash, &data_hash).hash(),
                None => meta_hash.clone(),
            },
            meta_hash,
            meta_bytes,
            data_hash,
        }
    }
    pub(crate) fn as_header(&self) -> Result<EventHeader, SerializationError> {
        Ok(
            EventHeader {
                raw: self.clone(),
                meta: rmps::from_read_ref(&self.meta_bytes)?,
            }
        )
    }
}

/// Describes what the event is and includes a structured object to represent it
#[derive(Debug, Clone)]
pub struct EventHeader
{
    pub raw: EventHeaderRaw,
    pub meta: Metadata,
}

impl EventHeader {
    pub fn hash(&self) -> Hash {
        self.raw.event_hash
    }
}

/// Represents an event that has not yet been stored anywhere
#[derive(Debug, Clone)]
pub struct EventData
{
    pub meta: Metadata,
    pub data_bytes: Option<Bytes>,
}

impl EventData
{
    #[allow(dead_code)]
    pub(crate) fn new(key: PrimaryKey, data: Bytes) -> EventData {        
        EventData {
            meta: Metadata::for_data(key),
            data_bytes: Some(data),
        }
    }

    pub(crate) fn as_header_raw(&self) -> Result<EventHeaderRaw, SerializationError> {
        let data_hash = match &self.data_bytes {
            Some(d) => Some(Hash::from_bytes(&d[..])),
            None => None,
        };
        let meta_bytes = Bytes::from(rmps::to_vec(&self.meta)?);
        let meta_hash = Hash::from_bytes(&meta_bytes[..]);

        Ok(
            EventHeaderRaw::new(meta_hash, meta_bytes, data_hash)
        )
    }

    pub(crate) fn as_header(&self) -> Result<EventHeader, SerializationError> {
        Ok(self.as_header_raw()?.as_header()?)
    }

    #[allow(dead_code)]
    pub(crate) fn with_core_metadata(mut self, core: CoreMetadata) -> Self {
        self.meta.core.push(core);
        self
    }
}