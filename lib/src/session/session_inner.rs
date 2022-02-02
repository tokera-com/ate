use super::*;
use crate::crypto::*;
use serde::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AteSessionInner {
    User(AteSessionUser),
    Sudo(AteSessionSudo),
}

impl AteSession for AteSessionInner {
    fn role<'a>(&'a self, purpose: &AteRolePurpose) -> Option<&'a AteGroupRole> {
        match self {
            AteSessionInner::User(a) => a.role(purpose),
            AteSessionInner::Sudo(a) => a.role(purpose),
        }
    }

    fn read_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a EncryptKey> + 'a> {
        match self {
            AteSessionInner::User(a) => a.read_keys(category),
            AteSessionInner::Sudo(a) => a.read_keys(category),
        }
    }

    fn write_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a PrivateSignKey> + 'a> {
        match self {
            AteSessionInner::User(a) => a.write_keys(category),
            AteSessionInner::Sudo(a) => a.write_keys(category),
        }
    }

    fn public_read_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a PublicEncryptKey> + 'a> {
        match self {
            AteSessionInner::User(a) => a.public_read_keys(category),
            AteSessionInner::Sudo(a) => a.public_read_keys(category),
        }
    }

    fn private_read_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a PrivateEncryptKey> + 'a> {
        match self {
            AteSessionInner::User(a) => a.private_read_keys(category),
            AteSessionInner::Sudo(a) => a.private_read_keys(category),
        }
    }

    fn broker_read<'a>(&'a self) -> Option<&'a PrivateEncryptKey> {
        match self {
            AteSessionInner::User(a) => a.broker_read(),
            AteSessionInner::Sudo(a) => a.broker_read(),
        }
    }

    fn broker_write<'a>(&'a self) -> Option<&'a PrivateSignKey> {
        match self {
            AteSessionInner::User(a) => a.broker_write(),
            AteSessionInner::Sudo(a) => a.broker_write(),
        }
    }

    fn identity<'a>(&'a self) -> &'a str {
        match self {
            AteSessionInner::User(a) => a.identity(),
            AteSessionInner::Sudo(a) => a.identity(),
        }
    }

    fn user<'a>(&'a self) -> &'a AteSessionUser {
        match self {
            AteSessionInner::User(a) => a.user(),
            AteSessionInner::Sudo(a) => a.user(),
        }
    }

    fn user_mut<'a>(&'a mut self) -> &'a mut AteSessionUser {
        match self {
            AteSessionInner::User(a) => a.user_mut(),
            AteSessionInner::Sudo(a) => a.user_mut(),
        }
    }

    fn uid<'a>(&'a self) -> Option<u32> {
        match self {
            AteSessionInner::User(a) => a.uid(),
            AteSessionInner::Sudo(a) => a.uid(),
        }
    }

    fn gid<'a>(&'a self) -> Option<u32> {
        None
    }

    fn clone_session(&self) -> Box<dyn AteSession> {
        Box::new(self.clone())
    }

    fn clone_inner(&self) -> AteSessionInner {
        self.clone()
    }

    fn properties<'a>(&'a self) -> Box<dyn Iterator<Item = &'a AteSessionProperty> + 'a> {
        match self {
            AteSessionInner::User(a) => a.properties(),
            AteSessionInner::Sudo(a) => a.properties(),
        }
    }

    fn append<'a, 'b>(
        &'a mut self,
        properties: Box<dyn Iterator<Item = &'b AteSessionProperty> + 'b>,
    ) {
        match self {
            AteSessionInner::User(a) => a.append(properties),
            AteSessionInner::Sudo(a) => a.append(properties),
        }
    }
}

impl std::fmt::Display for AteSessionInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        match self {
            AteSessionInner::User(a) => write!(f, "user: {}", a),
            AteSessionInner::Sudo(a) => write!(f, "sudo: {}", a),
        }?;
        write!(f, "]")
    }
}

impl From<AteSessionUser> for AteSessionInner {
    fn from(a: AteSessionUser) -> Self {
        AteSessionInner::User(a)
    }
}

impl From<AteSessionSudo> for AteSessionInner {
    fn from(a: AteSessionSudo) -> Self {
        AteSessionInner::Sudo(a)
    }
}