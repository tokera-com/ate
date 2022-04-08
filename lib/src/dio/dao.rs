#![allow(unused_imports)]
use fxhash::FxHashSet;
use tracing::{debug, warn};

use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Weak};
use std::sync::{Mutex, MutexGuard};

use crate::crypto::{EncryptedPrivateKey, PrivateSignKey};
use crate::{
    crypto::EncryptKey,
    session::{AteSession, AteSessionProperty},
};

use super::dio_mut::*;
use crate::crypto::AteHash;
use crate::dio::*;
use crate::error::*;
use crate::event::*;
use crate::header::*;
use crate::index::*;
use crate::meta::*;
use crate::spec::*;

use super::row::*;
pub use super::vec::DaoVec;

pub trait DaoObj {
    fn key(&self) -> &PrimaryKey;

    fn auth(&self) -> &MetaAuthorization;

    fn dio(&self) -> &Arc<Dio>;

    fn when_created(&self) -> u64;

    fn when_updated(&self) -> u64;
}

/// Represents a data object that will be represented as one or
/// more events on the redo-log and validated in the chain-of-trust.
///
/// Reading this object using none-mutable behavior will incur no IO
/// on the redo-log however if you edit the object you must commit it
/// to the `Dio` before it goes out of scope or the data will be lost
/// (in Debug mode this will even trigger an assert).
///
/// Metadata about the data object can also be accessed via this object
/// which allows you to read access rights, etc.
///
/// If you wish to actually modify the data you must first call the 'mut'
/// function on an open transaction, which will give you an object you
/// can modify
pub struct Dao<D> {
    dio: Arc<Dio>,
    pub(super) row_header: RowHeader,
    pub(super) row: Row<D>,
}

impl<D> Clone for Dao<D>
where
    D: Clone,
{
    fn clone(&self) -> Self {
        Dao {
            dio: self.dio.clone(),
            row_header: self.row_header.clone(),
            row: self.row.clone(),
        }
    }
}

impl<D> std::fmt::Debug for Dao<D>
where
    D: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.row.fmt(f)
    }
}

impl<D> PartialEq<Dao<D>>
for Dao<D>
where D: PartialEq<D>
{
    fn eq(&self, other: &Dao<D>) -> bool {
        if self.row.key.ne(&other.row.key) {
            return false;
        }
        if self.row_header.auth.ne(&other.row_header.auth) {
            return false;
        }
        if self.row_header.parent.ne(&other.row_header.parent) {
            return false;
        }
        self.row.data.eq(&other.row.data)
    }
}

impl<D> Eq
for Dao<D>
where D: Eq + PartialEq<Dao<D>>
{ }

impl<D> Dao<D> {
    pub(super) fn new(dio: &Arc<Dio>, row_header: RowHeader, row: Row<D>) -> Dao<D> {
        Dao {
            dio: Arc::clone(dio),
            row_header,
            row,
        }
    }

    pub fn take(self) -> D {
        self.row.data
    }

    pub fn parent(&self) -> Option<MetaCollection> {
        self.row_header.parent.as_ref().map(|a| a.vec.clone())
    }

    pub fn parent_id(&self) -> Option<PrimaryKey> {
        self.row_header
            .parent
            .as_ref()
            .map(|a| a.vec.parent_id.clone())
    }
}

impl<D> DaoObj for Dao<D> {
    fn auth(&self) -> &MetaAuthorization {
        &self.row_header.auth
    }

    fn dio(&self) -> &Arc<Dio> {
        &self.dio
    }

    fn key(&self) -> &PrimaryKey {
        &self.row.key
    }

    fn when_created(&self) -> u64 {
        self.row.created
    }

    fn when_updated(&self) -> u64 {
        self.row.updated
    }
}

impl<D> std::ops::Deref for Dao<D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.row.data
    }
}
