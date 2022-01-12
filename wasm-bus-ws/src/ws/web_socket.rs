use std::io;
use std::io::Read;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::watch;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::api::SendResult;
use crate::api::SocketState;
use wasm_bus::abi::*;

#[derive(Debug)]
pub struct WebSocket {
    pub(super) client: Arc<dyn crate::api::WebSocket>,
    pub(super) rx: mpsc::Receiver<Vec<u8>>,
    pub(super) state: watch::Receiver<SocketState>,
}

impl WebSocket {
    pub fn split(self) -> (SendHalf, RecvHalf) {
        (
            SendHalf {
                client: self.client,
                state: self.state,
            },
            RecvHalf { rx: self.rx },
        )
    }
}

#[derive(Debug, Clone)]
pub struct SendHalf {
    client: Arc<dyn crate::api::WebSocket>,
    state: watch::Receiver<SocketState>,
}

impl SendHalf {
    pub async fn wait_till_opened(&self) -> SocketState {
        let mut state = self.state.clone();
        while *state.borrow() == SocketState::Opening {
            if let Err(_) = state.changed().await {
                return SocketState::Closed;
            }
        }
        let ret = (*state.borrow()).clone();
        ret
    }

    pub async fn send(&self, data: Vec<u8>) -> io::Result<usize> {
        let state = self.wait_till_opened().await;
        if state != SocketState::Opened {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                format!("connection is not open (state={})", state).as_str(),
            ));
        }
        self.client
            .send(data)
            .await
            .map_err(|err| err.into_io_error())
            .map(|ret| match ret {
                SendResult::Success(a) => Ok(a),
                SendResult::Failed(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
            })?
    }

    pub fn blocking_send(&self, data: Vec<u8>) -> io::Result<usize> {
        if *self.state.borrow() != SocketState::Opened {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "connection is not open",
            ));
        }
        self.client
            .blocking_send(data)
            .map_err(|err| err.into_io_error())
            .map(|ret| match ret {
                SendResult::Success(a) => Ok(a),
                SendResult::Failed(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
            })?
    }
}

#[derive(Debug)]
pub struct RecvHalf {
    rx: mpsc::Receiver<Vec<u8>>,
}

impl RecvHalf {
    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        self.rx.recv().await
    }

    pub fn blocking_recv(&mut self) -> Option<Vec<u8>> {
        self.rx.blocking_recv()
    }
}
