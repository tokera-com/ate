use std::io;
use std::io::Read;
use std::io::Write;
#[cfg(feature = "tokio")]
use tokio::sync::mpsc;
#[cfg(not(feature = "tokio"))]
use std::sync::mpsc;

use crate::abi::*;
use crate::backend::ws::*;

#[derive(Debug)]
pub struct WebSocket {
    pub(super) task: Call,
    pub(super) rx: mpsc::Receiver<Vec<u8>>
}

impl WebSocket
{
    pub fn split(self) -> (SendHalf, RecvHalf)
    {
        (
            SendHalf { task: self.task },
            RecvHalf { rx: self.rx },
        )
    }
}

#[derive(Debug, Clone)]
pub struct SendHalf {
    task: Call
}

impl SendHalf {
    pub async fn send(&self, data: Vec<u8>) -> io::Result<()> {
        self.task.call(Send { data }).invoke().join().await
            .map_err(|err| err.into_io_error())
    }

    pub fn blocking_send(&self, data: Vec<u8>) -> io::Result<()> {
        self.task.call(Send { data }).invoke().join().wait()
            .map_err(|err| err.into_io_error())
    }
}

#[derive(Debug)]
pub struct RecvHalf {
    rx: mpsc::Receiver<Vec<u8>>
}

impl RecvHalf {
    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        //self.rx.recv().await
        self.rx.recv().ok()
    }

    pub fn blocking_recv(&mut self) -> Option<Vec<u8>> {
        self.rx.recv().ok()
    }
}