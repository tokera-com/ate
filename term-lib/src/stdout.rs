#![allow(unused_imports)]
#![allow(dead_code)]
use std::io::Write;
use std::ops::{Deref, DerefMut};
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::cconst::*;
use super::common::*;
use super::fd::*;
use super::tty::*;

#[derive(Debug, Clone)]
pub struct Stdout {
    pub fd: Fd,
}

impl Deref for Stdout {
    type Target = Fd;

    fn deref(&self) -> &Self::Target {
        &self.fd
    }
}

impl DerefMut for Stdout {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fd
    }
}

impl Stdout {
    pub fn new(mut fd: Fd) -> Stdout {
        fd.flag = FdFlag::Stdout(fd.is_tty());
        Stdout {
            fd
        }
    }

    pub fn fd(&self) -> Fd {
        self.fd.clone()
    }

    pub async fn draw(&mut self, data: &str) {
        if let Err(err) = self.fd.write(data.as_bytes()).await {
            warn!("stdout-err: {}", err);
        }
    }
}
