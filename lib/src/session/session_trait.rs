#[allow(unused_imports)]
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::session_sudo::*;
use super::session_user::*;
use super::AteGroupRole;
use super::AteRolePurpose;
use super::AteSessionGroup;
use super::AteSessionInner;
use super::AteSessionProperty;
use super::AteSessionType;
use crate::crypto::*;

pub type SessionToken = Option<EncryptedSecureData<EncryptKey>>;

/// Sessions hold facts about the user that give them certains
/// rights and abilities to view data within the chain-of-trust.
///
/// For instance - to see encrypted data for specific users they
/// must insert their `EncryptKey` into this session before
/// accessing the chain via a `Dio`.
///
/// Another example is the ability to write data. For certain
/// records within the tree if they have been protected with
/// write protection then you must insert your `PrivateKey`
/// into the session before you attempt to insert or modify these
/// records.
///
/// Sessions are never cached and only exist in memory for the
/// duration that you use them for security reasons.
pub trait AteSession: Send + Sync + std::fmt::Display {
    fn role<'a>(&'a self, purpose: &AteRolePurpose) -> Option<&'a AteGroupRole>;

    fn read_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a EncryptKey> + 'a>;

    fn write_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a PrivateSignKey> + 'a>;

    fn public_read_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a PublicEncryptKey> + 'a>;

    fn private_read_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a PrivateEncryptKey> + 'a>;

    fn broker_read<'a>(&'a self) -> Option<&'a PrivateEncryptKey>;

    fn broker_write<'a>(&'a self) -> Option<&'a PrivateSignKey>;

    fn identity<'a>(&'a self) -> &'a str;

    fn user<'a>(&'a self) -> &'a AteSessionUser;

    fn user_mut<'a>(&'a mut self) -> &'a mut AteSessionUser;

    fn uid<'a>(&'a self) -> Option<u32>;

    fn gid<'a>(&'a self) -> Option<u32>;

    fn properties<'a>(&'a self) -> Box<dyn Iterator<Item = &'a AteSessionProperty> + 'a>;

    fn append<'a, 'b>(
        &'a mut self,
        properties: Box<dyn Iterator<Item = &'b AteSessionProperty> + 'b>,
    );

    fn clone_session(&self) -> Box<dyn AteSession>;

    fn clone_inner(&self) -> AteSessionInner;
}

impl std::fmt::Debug
for dyn AteSession
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl From<AteSessionUser> for Box<dyn AteSession> {
    fn from(session: AteSessionUser) -> Self {
        Box::new(session)
    }
}

impl From<AteSessionSudo> for Box<dyn AteSession> {
    fn from(session: AteSessionSudo) -> Self {
        Box::new(session)
    }
}

impl From<AteSessionGroup> for Box<dyn AteSession> {
    fn from(session: AteSessionGroup) -> Self {
        Box::new(session)
    }
}

impl From<AteSessionInner> for Box<dyn AteSession> {
    fn from(session: AteSessionInner) -> Self {
        Box::new(session)
    }
}

impl From<AteSessionType> for Box<dyn AteSession> {
    fn from(session: AteSessionType) -> Self {
        Box::new(session)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AteSessionKeyCategory {
    UpperKeys,
    UserKeys,
    SudoKeys,
    GroupKeys,
    BrokerKeys,
    NonGroupKeys,
    AllKeys,
}

impl AteSessionKeyCategory {
    pub fn includes_user_keys(&self) -> bool {
        match self {
            AteSessionKeyCategory::UserKeys => true,
            AteSessionKeyCategory::NonGroupKeys => true,
            AteSessionKeyCategory::AllKeys => true,
            _ => false,
        }
    }
    pub fn includes_sudo_keys(&self) -> bool {
        match self {
            AteSessionKeyCategory::SudoKeys => true,
            AteSessionKeyCategory::NonGroupKeys => true,
            AteSessionKeyCategory::AllKeys => true,
            _ => false,
        }
    }
    pub fn includes_group_keys(&self) -> bool {
        match self {
            AteSessionKeyCategory::GroupKeys => true,
            AteSessionKeyCategory::AllKeys => true,
            _ => false,
        }
    }
    pub fn includes_broker_keys(&self) -> bool {
        match self {
            AteSessionKeyCategory::BrokerKeys => true,
            AteSessionKeyCategory::AllKeys => true,
            _ => false,
        }
    }
}
