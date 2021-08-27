use serde::*;
use super::*;
use crate::crypto::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AteSessionType
{
    User(AteSessionUser),
    Sudo(AteSessionSudo),
    Group(AteSessionGroup),
}

impl AteSession
for AteSessionType
{
    fn role<'a>(&'a self, purpose: &AteRolePurpose) -> Option<&'a AteGroupRole> {
        match self {
            AteSessionType::User(a) => a.role(purpose),
            AteSessionType::Sudo(a) => a.role(purpose),
            AteSessionType::Group(a) => a.role(purpose)
        }
    }

    fn read_keys<'a>(&'a self) -> Box<dyn Iterator<Item = &'a EncryptKey> + 'a> {
        match self {
            AteSessionType::User(a) => a.read_keys(),
            AteSessionType::Sudo(a) => a.read_keys(),
            AteSessionType::Group(a) => a.read_keys()
        }
    }

    fn write_keys<'a>(&'a self) -> Box<dyn Iterator<Item = &'a PrivateSignKey> + 'a> {
        match self {
            AteSessionType::User(a) => a.write_keys(),
            AteSessionType::Sudo(a) => a.write_keys(),
            AteSessionType::Group(a) => a.write_keys()
        }
    }

    fn public_read_keys<'a>(&'a self) -> Box<dyn Iterator<Item = &'a PublicEncryptKey> + 'a> {
        match self {
            AteSessionType::User(a) => a.public_read_keys(),
            AteSessionType::Sudo(a) => a.public_read_keys(),
            AteSessionType::Group(a) => a.public_read_keys()
        }
    }

    fn private_read_keys<'a>(&'a self) -> Box<dyn Iterator<Item = &'a PrivateEncryptKey> + 'a> {
        match self {
            AteSessionType::User(a) => a.private_read_keys(),
            AteSessionType::Sudo(a) => a.private_read_keys(),
            AteSessionType::Group(a) => a.private_read_keys()
        }
    }

    fn identity<'a>(&'a self) -> &'a str {
        match self {
            AteSessionType::User(a) => a.identity(),
            AteSessionType::Sudo(a) => a.identity(),
            AteSessionType::Group(a) => a.identity()
        }
    }

    fn uid<'a>(&'a self) -> Option<u32> {
        match self {
            AteSessionType::User(a) => a.uid(),
            AteSessionType::Sudo(a) => a.uid(),
            AteSessionType::Group(a) => a.uid()
        }
    }

    fn gid<'a>(&'a self) -> Option<u32> {
        match self {
            AteSessionType::User(a) => a.gid(),
            AteSessionType::Sudo(a) => a.gid(),
            AteSessionType::Group(a) => a.gid()
        }
    }

    fn clone_session(&self) -> Box<dyn AteSession> {
        Box::new(self.clone())
    }

    fn properties<'a>(&'a self) -> Box<dyn Iterator<Item = &'a AteSessionProperty> + 'a> {
        match self {
            AteSessionType::User(a) => a.properties(),
            AteSessionType::Sudo(a) => a.properties(),
            AteSessionType::Group(a) => a.properties()
        }
    }

    fn append<'a, 'b>(&'a mut self, properties: Box<dyn Iterator<Item = &'b AteSessionProperty> + 'b>) {
        match self {
            AteSessionType::User(a) => a.append(properties),
            AteSessionType::Sudo(a) => a.append(properties),
            AteSessionType::Group(a) => a.append(properties)
        }
    }
}

impl std::fmt::Display
for AteSessionType
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        match self {
            AteSessionType::User(a) => write!(f, "user: {}", a),
            AteSessionType::Sudo(a) => write!(f, "sudo: {}", a),
            AteSessionType::Group(a) => write!(f, "group: {}", a)
        }?;
        write!(f, "]")
    }
}

impl From<AteSessionInner>
for AteSessionType
{
    fn from(a: AteSessionInner) -> Self {
        match a {
            AteSessionInner::User(a) => AteSessionType::User(a),
            AteSessionInner::Sudo(a) => AteSessionType::Sudo(a),
        }
    }
}

impl From<AteSessionUser>
for AteSessionType
{
    fn from(a: AteSessionUser) -> Self {
        AteSessionType::User(a)
    }
}

impl From<AteSessionSudo>
for AteSessionType
{
    fn from(a: AteSessionSudo) -> Self {
        AteSessionType::Sudo(a)
    }
}

impl From<AteSessionGroup>
for AteSessionType
{
    fn from(a: AteSessionGroup) -> Self {
        AteSessionType::Group(a)
    }
}