#![allow(unused_imports)]
use crate::crypto::InitializationVector;
use crate::engine::timeout as tokio_timeout;
use bytes::BytesMut;
use error_chain::bail;
use std::collections::VecDeque;
use std::fs::File;
use std::net::SocketAddr;
use std::result::Result;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::io::Error as TError;
use std::io::ErrorKind as TErrorKind;
#[cfg(feature = "enable_full")]
use tokio::net::tcp::OwnedReadHalf;
#[cfg(feature = "enable_full")]
use tokio::net::tcp::OwnedWriteHalf;
#[cfg(feature = "enable_full")]
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use wasm_bus_ws::prelude::RecvHalf as WasmRecvHalf;
use wasm_bus_ws::prelude::SendHalf as WasmSendHalf;
use wasm_bus_ws::prelude::WebSocket as WasmWebSocket;
use bytes::Bytes;
use ate_mio::comms::StreamReceiver;
use ate_mio::comms::StreamTransmitter;

use crate::comms::PacketData;
use crate::crypto::EncryptKey;

#[cfg(feature = "enable_server")]
use {
    hyper_tungstenite::hyper::upgrade::Upgraded as HyperUpgraded,
    hyper_tungstenite::tungstenite::Error as HyperError,
    hyper_tungstenite::tungstenite::Message as HyperMessage,
    hyper_tungstenite::WebSocketStream as HyperWebSocket,
};

#[cfg(feature = "enable_full")]
use {
    tokio::io::{AsyncReadExt, AsyncWriteExt},
    tokio_tungstenite::{tungstenite::Message, WebSocketStream},
};

use crate::error::*;

use super::NodeId;
use super::protocol::MessageProtocolApi;

#[derive(Debug, Clone, Copy)]
pub enum StreamProtocol {
    Tcp,
    WebSocket,
    SecureWebSocket,
}

impl std::str::FromStr for StreamProtocol {
    type Err = CommsError;

    fn from_str(s: &str) -> Result<StreamProtocol, CommsError> {
        let ret = match s {
            "tcp" => StreamProtocol::Tcp,
            "ws" => StreamProtocol::WebSocket,
            "wss" => StreamProtocol::SecureWebSocket,
            _ => {
                bail!(CommsErrorKind::UnsupportedProtocolError(s.to_string()));
            }
        };
        Ok(ret)
    }
}

impl StreamProtocol {
    pub fn to_scheme(&self) -> String {
        let ret = match self {
            StreamProtocol::Tcp => "tcp",
            StreamProtocol::WebSocket => "ws",
            StreamProtocol::SecureWebSocket => "wss",
        };
        ret.to_string()
    }

    pub fn to_string(&self) -> String {
        self.to_scheme()
    }

    pub fn default_port(&self) -> u16 {
        match self {
            StreamProtocol::Tcp => 5000,
            StreamProtocol::WebSocket => 80,
            StreamProtocol::SecureWebSocket => 443,
        }
    }

    pub fn is_tcp(&self) -> bool {
        match self {
            StreamProtocol::Tcp => true,
            StreamProtocol::WebSocket => false,
            StreamProtocol::SecureWebSocket => false,
        }
    }

    pub fn is_web_socket(&self) -> bool {
        match self {
            StreamProtocol::Tcp => false,
            StreamProtocol::WebSocket => true,
            StreamProtocol::SecureWebSocket => true,
        }
    }
}

impl std::fmt::Display for StreamProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_scheme())
    }
}

pub trait AsyncStream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + Sync {}

impl<T> AsyncStream for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + Sync
{}

impl std::fmt::Debug for dyn AsyncStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("async-stream")
    }
}

#[derive(Debug)]
pub enum Stream {
    #[cfg(feature = "enable_full")]
    Tcp(TcpStream),
    #[cfg(feature = "enable_full")]
    WebSocket(WebSocketStream<TcpStream>, StreamProtocol),
    #[cfg(feature = "enable_server")]
    HyperWebSocket(HyperWebSocket<HyperUpgraded>, StreamProtocol),
    ViaStream(Box<dyn AsyncStream + 'static>, StreamProtocol),
    ViaQueue(
        mpsc::Sender<Vec<u8>>,
        mpsc::Receiver<Vec<u8>>,
        StreamProtocol,
    ),
    ViaFile(std::fs::File, StreamProtocol),
    WasmWebSocket(WasmWebSocket),
}

impl StreamProtocol {
    pub fn make_url(
        &self,
        domain: String,
        port: u16,
        path: String,
    ) -> Result<url::Url, url::ParseError> {
        let scheme = self.to_scheme();
        let input = match port {
            a if a == self.default_port() => match path.starts_with("/") {
                true => format!("{}://{}:{}{}", scheme, domain, port, path),
                false => format!("{}://{}:{}/{}", scheme, domain, port, path),
            },
            _ => match path.starts_with("/") {
                true => format!("{}://{}{}", scheme, domain, path),
                false => format!("{}://{}/{}", scheme, domain, path),
            },
        };
        url::Url::parse(input.as_str())
    }

    pub fn parse(url: &url::Url) -> Result<StreamProtocol, CommsError> {
        let scheme = url.scheme().to_string().to_lowercase();
        StreamProtocol::from_str(scheme.as_str())
    }
}

#[derive(Debug)]
pub struct StreamRx {
    inner: StreamRxInner,
    protocol: StreamProtocol,
    version: Box<dyn MessageProtocolApi + 'static>,
}

#[derive(Debug)]
pub enum StreamRxInner {
    #[cfg(feature = "enable_full")]
    Tcp(OwnedReadHalf),
    #[cfg(feature = "enable_full")]
    WebSocket(futures_util::stream::SplitStream<WebSocketStream<TcpStream>>),
    #[cfg(feature = "enable_server")]
    HyperWebSocket(futures_util::stream::SplitStream<HyperWebSocket<HyperUpgraded>>),
    ViaStream(Arc<tokio::sync::Mutex<Box<dyn AsyncStream + 'static>>>),
    ViaQueue(mpsc::Receiver<Vec<u8>>),
    ViaFile(Arc<std::sync::Mutex<std::fs::File>>),
    WasmWebSocket(WasmRecvHalf),
}

#[cfg(feature = "enable_full")]
impl From<OwnedReadHalf>
for StreamRx
{
    fn from(inner: OwnedReadHalf) -> Self {
        Self::new(StreamRxInner::Tcp(inner), StreamProtocol::Tcp)
    }
}

#[cfg(feature = "enable_full")]
impl From<futures_util::stream::SplitStream<WebSocketStream<TcpStream>>>
for StreamRx
{
    fn from(inner: futures_util::stream::SplitStream<WebSocketStream<TcpStream>>) -> Self {
        Self::new(StreamRxInner::WebSocket(inner), StreamProtocol::WebSocket)
    }
}

#[cfg(feature = "enable_server")]
impl From<futures_util::stream::SplitStream<HyperWebSocket<HyperUpgraded>>>
for StreamRx
{
    fn from(inner: futures_util::stream::SplitStream<HyperWebSocket<HyperUpgraded>>) -> Self {
        Self::new(StreamRxInner::HyperWebSocket(inner), StreamProtocol::WebSocket)
    }
}

impl From<WasmRecvHalf>
for StreamRx
{
    fn from(inner: WasmRecvHalf) -> Self {
        Self::new(StreamRxInner::WasmWebSocket(inner), StreamProtocol::WebSocket)
    }
}

#[derive(Debug)]
pub struct StreamTx {
    inner: StreamTxInner,
    #[allow(dead_code)]
    protocol: StreamProtocol,
    version: Box<dyn MessageProtocolApi + 'static>,
}

#[derive(Debug)]
pub enum StreamTxInner {
    #[cfg(feature = "enable_full")]
    Tcp(OwnedWriteHalf),
    #[cfg(feature = "enable_full")]
    WebSocket(futures_util::stream::SplitSink<WebSocketStream<TcpStream>, Message>),
    #[cfg(feature = "enable_server")]
    HyperWebSocket(futures_util::stream::SplitSink<HyperWebSocket<HyperUpgraded>, HyperMessage>),
    ViaStream(Arc<tokio::sync::Mutex<Box<dyn AsyncStream + 'static>>>),
    ViaQueue(mpsc::Sender<Vec<u8>>),
    ViaFile(Arc<std::sync::Mutex<std::fs::File>>),
    WasmWebSocket(WasmSendHalf),
}

#[cfg(feature = "enable_full")]
impl From<OwnedWriteHalf>
for StreamTx
{
    fn from(inner: OwnedWriteHalf) -> Self {
        Self::new(StreamTxInner::Tcp(inner), StreamProtocol::Tcp)
    }
}

#[cfg(feature = "enable_full")]
impl From<futures_util::stream::SplitSink<WebSocketStream<TcpStream>, Message>>
for StreamTx
{
    fn from(inner: futures_util::stream::SplitSink<WebSocketStream<TcpStream>, Message>) -> Self {
        Self::new(StreamTxInner::WebSocket(inner), StreamProtocol::WebSocket)
    }
}

#[cfg(feature = "enable_server")]
impl From<futures_util::stream::SplitSink<HyperWebSocket<HyperUpgraded>, HyperMessage>>
for StreamTx
{
    fn from(inner: futures_util::stream::SplitSink<HyperWebSocket<HyperUpgraded>, HyperMessage>) -> Self {
        Self::new(StreamTxInner::HyperWebSocket(inner), StreamProtocol::WebSocket)
    }
}

impl From<WasmSendHalf>
for StreamTx
{
    fn from(inner: WasmSendHalf) -> Self {
        Self::new(StreamTxInner::WasmWebSocket(inner), StreamProtocol::WebSocket)
    }
}

impl Stream {
    pub fn split(self) -> (StreamRx, StreamTx) {
        match self {
            #[cfg(feature = "enable_full")]
            Stream::Tcp(a) => {
                let (rx, tx) = a.into_split();
                (StreamRx::new(StreamRxInner::Tcp(rx), StreamProtocol::Tcp),
                 StreamTx::new(StreamTxInner::Tcp(tx), StreamProtocol::Tcp))
            }
            #[cfg(feature = "enable_full")]
            Stream::WebSocket(a, p) => {
                use futures_util::StreamExt;
                let (tx, rx) = a.split();
                (StreamRx::new(StreamRxInner::WebSocket(rx), p),
                 StreamTx::new(StreamTxInner::WebSocket(tx), p))
            }
            #[cfg(feature = "enable_server")]
            Stream::HyperWebSocket(a, p) => {
                use futures_util::StreamExt;
                let (tx, rx) = a.split();
                (StreamRx::new(StreamRxInner::HyperWebSocket(rx), p),
                 StreamTx::new(StreamTxInner::HyperWebSocket(tx), p))
            }
            Stream::ViaStream(a, p) => {
                let a = Arc::new(tokio::sync::Mutex::new(a));
                let b = Arc::clone(&a);
                (StreamRx::new(StreamRxInner::ViaStream(a), p),
                 StreamTx::new(StreamTxInner::ViaStream(b), p))
            }
            Stream::ViaQueue(a, b, p) => {
                (StreamRx::new(StreamRxInner::ViaQueue(b), p),
                 StreamTx::new(StreamTxInner::ViaQueue(a), p))
            },
            Stream::ViaFile(a, p) => {
                let rx = Arc::new(std::sync::Mutex::new(a));
                let tx = Arc::clone(&rx);
                (StreamRx::new(StreamRxInner::ViaFile(rx), p),
                 StreamTx::new(StreamTxInner::ViaFile(tx), p))
            }
            Stream::WasmWebSocket(a) => {
                let (tx, rx) = a.split();
                (StreamRx::new(StreamRxInner::WasmWebSocket(rx), StreamProtocol::WebSocket),
                 StreamTx::new(StreamTxInner::WasmWebSocket(tx), StreamProtocol::WebSocket))
            }
        }
    }

    #[cfg(feature = "enable_server")]
    pub async fn upgrade_server(
        self,
        protocol: StreamProtocol,
        timeout: Duration,
    ) -> Result<Stream, CommsError> {
        debug!("tcp-protocol-upgrade(server): {}", protocol);

        let ret = match self {
            #[cfg(feature = "enable_full")]
            Stream::Tcp(a) => match protocol {
                StreamProtocol::Tcp => Stream::Tcp(a),
                StreamProtocol::WebSocket | StreamProtocol::SecureWebSocket => {
                    let wait = tokio_tungstenite::accept_async(a);
                    let socket = tokio_timeout(timeout, wait).await??;
                    Stream::WebSocket(socket, protocol)
                }
            },
            #[cfg(feature = "enable_full")]
            Stream::WebSocket(a, p) => Stream::WebSocket(a, p),
            #[cfg(feature = "enable_server")]
            Stream::HyperWebSocket(a, p) => Stream::HyperWebSocket(a, p),
            Stream::ViaStream(a, p) => Stream::ViaStream(a, p),
            Stream::ViaQueue(a, b, p) => Stream::ViaQueue(a, b, p),
            Stream::ViaFile(a, p) => Stream::ViaFile(a, p),
            Stream::WasmWebSocket(a) => Stream::WasmWebSocket(a),
        };

        Ok(ret)
    }

    #[allow(dead_code)]
    #[allow(unused_variables)]
    pub async fn upgrade_client(self, protocol: StreamProtocol) -> Result<Stream, CommsError> {
        debug!("tcp-protocol-upgrade(client): {}", protocol);

        let ret = match self {
            #[cfg(feature = "enable_full")]
            Stream::Tcp(a) => match protocol {
                StreamProtocol::Tcp => Stream::Tcp(a),
                StreamProtocol::WebSocket | StreamProtocol::SecureWebSocket => {
                    let port = match protocol {
                        StreamProtocol::SecureWebSocket => 443,
                        _ => 80
                    };
                    let url = StreamProtocol::WebSocket.make_url(
                        "localhost".to_string(),
                        port,
                        "/".to_string(),
                    )?;
                    let mut request = tokio_tungstenite::tungstenite::http::Request::new(());
                    *request.uri_mut() =
                        tokio_tungstenite::tungstenite::http::Uri::from_str(url.as_str())?;
                    let (stream, response) = tokio_tungstenite::client_async(request, a).await?;
                    if response.status().is_client_error() {
                        bail!(CommsErrorKind::WebSocketInternalError(format!(
                            "HTTP error while performing WebSocket handshack - status-code={}",
                            response.status().as_u16()
                        )));
                    }
                    Stream::WebSocket(stream, protocol)
                }
            },
            #[cfg(feature = "enable_full")]
            Stream::WebSocket(a, p) => Stream::WebSocket(a, p),
            #[cfg(feature = "enable_server")]
            Stream::HyperWebSocket(a, p) => Stream::HyperWebSocket(a, p),
            Stream::ViaStream(a, p) => Stream::ViaStream(a, p),
            Stream::ViaQueue(a, b, p) => Stream::ViaQueue(a, b, p),
            Stream::ViaFile(a, p) => Stream::ViaFile(a, p),
            Stream::WasmWebSocket(a) => Stream::WasmWebSocket(a),
        };
        Ok(ret)
    }

    #[allow(dead_code)]
    pub fn protocol(&self) -> StreamProtocol {
        match self {
            #[cfg(feature = "enable_full")]
            Stream::Tcp(_) => StreamProtocol::Tcp,
            #[cfg(feature = "enable_full")]
            Stream::WebSocket(_, p) => p.clone(),
            #[cfg(feature = "enable_server")]
            Stream::HyperWebSocket(_, p) => p.clone(),
            Stream::ViaStream(_, p) => p.clone(),
            Stream::ViaQueue(_, _, p) => p.clone(),
            Stream::ViaFile(_, p) => p.clone(),
            Stream::WasmWebSocket(_) => StreamProtocol::WebSocket,
        }
    }
}

impl StreamTx {
    pub fn new(inner: StreamTxInner, proto: StreamProtocol) -> Self {
        Self {
            inner,
            protocol: proto,
            version: Box::new(super::protocol::v1::MessageProtocol::default()),
        }
    }

    pub fn change_protocol_version(&mut self, version: Box<dyn MessageProtocolApi + 'static>) {
        self.version = version;
    }

    pub async fn close(
        &mut self
    ) -> Result<(), tokio::io::Error>
    {
        let _ = self.version.send_close(&mut self.inner).await;
        self.inner.close().await
    }

    pub async fn write_with_fixed_16bit_header(
        &mut self,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> {
        self.version.write_with_fixed_16bit_header(&mut self.inner, buf, delay_flush).await
    }

    #[allow(unused_variables)]
    pub async fn write_with_fixed_32bit_header(
        &mut self,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<u64, tokio::io::Error> {
        self.version.write_with_fixed_32bit_header(&mut self.inner, buf, delay_flush).await
    }

    #[cfg(feature = "enable_server")]
    fn conv_error_kind(err: &HyperError) -> tokio::io::ErrorKind {
        match err {
            HyperError::AlreadyClosed => tokio::io::ErrorKind::ConnectionAborted,
            HyperError::ConnectionClosed => tokio::io::ErrorKind::ConnectionAborted,
            HyperError::Io(io) => io.kind(),
            _ => tokio::io::ErrorKind::Other,
        }
    }

    pub async fn send(
        &mut self,
        wire_encryption: &Option<EncryptKey>,
        data: &[u8],
    ) -> Result<u64, tokio::io::Error> {
        self.version.send(&mut self.inner, wire_encryption, data).await
    }
}

impl StreamTxInner {
    pub async fn close(
        &mut self
    ) -> Result<(), tokio::io::Error>
    {
        match self {
            #[cfg(feature = "enable_full")]
            StreamTxInner::Tcp(a) => {
                let _ = a.flush().await;
                a.shutdown().await?;
            }
            #[cfg(feature = "enable_full")]
            StreamTxInner::WebSocket(a) => {
                use futures_util::SinkExt;
                let _ = a.flush().await;
                a.close().await
                    .map_err(|err| tokio::io::Error::new(tokio::io::ErrorKind::Other, err))?;
            }
            #[cfg(feature = "enable_server")]
            StreamTxInner::HyperWebSocket(a) => {
                use futures_util::SinkExt;
                let _ = a.flush().await;
                a.close().await
                    .map_err(|err| tokio::io::Error::new(tokio::io::ErrorKind::Other, err))?;
            }
            StreamTxInner::ViaStream(a) => {
                use tokio::io::AsyncWriteExt;
                let mut a = a.lock().await;
                let _ = a.flush().await;
                a.shutdown().await?;
            }
            StreamTxInner::ViaQueue(_) => {
            }
            StreamTxInner::ViaFile(_) => {
            }
            StreamTxInner::WasmWebSocket(a) => {
                a.close().await?;
            }
        }
        Ok(())
    }

    #[allow(unused_variables)]
    pub async fn write_all(
        &mut self,
        buf: &'_ [u8],
        delay_flush: bool,
    ) -> Result<(), tokio::io::Error> {
        #[allow(unused_mut)]
        match self {
            #[cfg(feature = "enable_full")]
            StreamTxInner::Tcp(a) => {
                a.write_all(buf).await?;
            }
            #[cfg(feature = "enable_full")]
            StreamTxInner::WebSocket(a) => {
                use futures_util::SinkExt;
                if delay_flush {
                    match a.feed(Message::binary(buf)).await {
                        Ok(a) => a,
                        Err(err) => {
                            return Err(tokio::io::Error::new(
                                tokio::io::ErrorKind::Other,
                                format!("Failed to feed data into websocket - {}", err.to_string()),
                            ));
                        }
                    }
                } else {
                    match a.send(Message::binary(buf)).await {
                        Ok(a) => a,
                        Err(err) => {
                            return Err(tokio::io::Error::new(
                                tokio::io::ErrorKind::Other,
                                format!("Failed to feed data into websocket - {}", err.to_string()),
                            ));
                        }
                    }
                }
            }
            #[cfg(feature = "enable_server")]
            StreamTxInner::HyperWebSocket(a) => {
                use futures_util::SinkExt;
                if delay_flush {
                    match a.feed(HyperMessage::binary(buf)).await {
                        Ok(a) => a,
                        Err(err) => {
                            let kind = StreamTx::conv_error_kind(&err);
                            return Err(tokio::io::Error::new(
                                kind,
                                format!("Failed to feed data into websocket - {}", err.to_string()),
                            ));
                        }
                    }
                } else {
                    match a.send(HyperMessage::binary(buf)).await {
                        Ok(a) => a,
                        Err(err) => {
                            let kind = StreamTx::conv_error_kind(&err);
                            return Err(tokio::io::Error::new(
                                kind,
                                format!("Failed to feed data into websocket - {}", err.to_string()),
                            ));
                        }
                    }
                }
            }
            StreamTxInner::ViaStream(a) => {
                use tokio::io::AsyncWriteExt;
                let mut a = a.lock().await;
                if buf.len() > u32::MAX as usize {
                    return Err(tokio::io::Error::new(
                        tokio::io::ErrorKind::InvalidData,
                        format!(
                            "Data is to big to write (len={}, max={})",
                            buf.len(),
                            u32::MAX
                        ),
                    ));
                }
                a.write_all(&buf[..]).await?;
            }
            StreamTxInner::ViaQueue(a) => {
                let buf = buf.to_vec();
                match a.send(buf).await {
                    Ok(a) => a,
                    Err(err) => {
                        return Err(tokio::io::Error::new(
                            tokio::io::ErrorKind::Other,
                            format!("Failed to send data on pipe/queue - {}", err.to_string()),
                        ));
                    }
                }
            }
            StreamTxInner::ViaFile(a) => {
                use std::io::Write;
                let mut a = a.lock().unwrap();
                a.write_all(buf)?;
            }
            StreamTxInner::WasmWebSocket(a) => {
                a.send(buf.to_vec()).await?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct StreamTxChannel {
    tx: StreamTx,
    pub(crate) wire_encryption: Option<EncryptKey>,
}

impl StreamTxChannel {
    pub fn new(tx: StreamTx, wire_encryption: Option<EncryptKey>) -> StreamTxChannel {
        StreamTxChannel {
            tx,
            wire_encryption,
        }
    }

    pub async fn send(&mut self, data: &[u8]) -> Result<u64, tokio::io::Error> {
        self.tx.send(&self.wire_encryption, data).await
    }

    pub async fn close(&mut self) -> Result<(), tokio::io::Error> {
        self.tx.close().await
    }
}

impl StreamRxInner {

    pub async fn recv(&mut self) -> Result<Vec<u8>, tokio::io::Error> {
        #[allow(unused_variables)]
        let ret = match self {
            #[cfg(feature = "enable_full")]
            StreamRxInner::Tcp(a) => {
                let mut bytes = vec![0u8; 4096];
                let n = a.read(&mut bytes).await?;
                if n <= 0 {
                    return Ok(vec![]);
                }
                bytes[0..n].to_vec()
            }
            #[cfg(feature = "enable_full")]
            StreamRxInner::WebSocket(a) => {
                use futures_util::StreamExt;
                match a.next().await {
                    Some(a) => {
                        let msg = match a {
                            Ok(a) => a,
                            Err(err) => {
                                return Err(tokio::io::Error::new(
                                    tokio::io::ErrorKind::BrokenPipe,
                                    format!(
                                        "Failed to receive data from websocket - {}",
                                        err.to_string()
                                    ),
                                ));
                            }
                        };
                        match msg {
                            Message::Binary(a) => a,
                            _ => {
                                return Err(tokio::io::Error::new(tokio::io::ErrorKind::BrokenPipe, format!("Failed to receive data from websocket as the message was the wrong type")));
                            }
                        }
                    }
                    None => {
                        return Err(tokio::io::Error::new(
                            tokio::io::ErrorKind::BrokenPipe,
                            format!("Failed to receive data from websocket"),
                        ));
                    }
                }
            }
            #[cfg(feature = "enable_server")]
            StreamRxInner::HyperWebSocket(a) => {
                use futures_util::StreamExt;
                match a.next().await {
                    Some(a) => {
                        let msg = match a {
                            Ok(a) => a,
                            Err(err) => {
                                return Err(tokio::io::Error::new(
                                    tokio::io::ErrorKind::BrokenPipe,
                                    format!(
                                        "Failed to receive data from websocket - {}",
                                        err.to_string()
                                    ),
                                ));
                            }
                        };
                        match msg {
                            HyperMessage::Binary(a) => a,
                            _ => {
                                return Err(tokio::io::Error::new(tokio::io::ErrorKind::BrokenPipe, format!("Failed to receive data from websocket as the message was the wrong type")));
                            }
                        }
                    }
                    None => {
                        return Err(tokio::io::Error::new(
                            tokio::io::ErrorKind::BrokenPipe,
                            format!("Failed to receive data from websocket"),
                        ));
                    }
                }
            }
            StreamRxInner::ViaStream(a) => {
                use tokio::io::AsyncReadExt;
                let mut a = a.lock().await;
                let mut ret = bytes::BytesMut::new();
                loop {
                    let mut buf = [0u8; 16384];
                    let n = a.read(&mut buf).await?;
                    if n > 0 {
                        ret.extend_from_slice(&buf[..n]);
                    } else {
                        break;
                    }
                }
                ret.to_vec()
            }
            StreamRxInner::ViaQueue(a) => match a.recv().await {
                Some(a) => a,
                None => {
                    return Err(tokio::io::Error::new(
                        tokio::io::ErrorKind::BrokenPipe,
                        format!("Failed to receive data from pipe/queue"),
                    ));
                }
            },
            StreamRxInner::ViaFile(a) => {
                use std::io::Read;
                let mut ret;
                loop {
                    let a = Arc::clone(a);
                    ret = crate::engine::TaskEngine::spawn_blocking(move || {
                        let mut data = Vec::new();
                        let mut temp = [0u8; 8192];
                        loop {
                            let mut file = a.lock().unwrap();
                            let nread = match file.read(&mut temp) {
                                Ok(a) => a,
                                Err(err) if err.kind() == tokio::io::ErrorKind::WouldBlock => {
                                    break;
                                }
                                Err(err) => {
                                    return Err(err);
                                }
                            };
                            drop(file);
                            if nread == 0 {
                                break;
                            }
                            data.extend_from_slice(&temp[..nread]);
                        }
                        Ok(data)
                    })
                    .await?;

                    if ret.len() <= 0 {
                        tokio::task::yield_now().await;
                        continue;
                    }

                    break;
                }
                ret
            }
            StreamRxInner::WasmWebSocket(a) => match a.recv().await {
                Some(a) => a,
                None => {
                    return Err(tokio::io::Error::new(
                        tokio::io::ErrorKind::BrokenPipe,
                        format!("Failed to receive data from web assembly socket"),
                    ));
                }
            },
        };
        Ok(ret)
    }
}

#[async_trait::async_trait]
pub trait StreamReader
{
    async fn read_buf_with_header(&mut self, wire_encryption: &Option<EncryptKey>, total_read: &mut u64) -> Result<Vec<u8>, TError>;
}

impl StreamRx {
    pub fn new(inner: StreamRxInner, proto: StreamProtocol) -> Self {
        Self {
            inner,
            protocol: proto,
            version: Box::new(super::protocol::v1::MessageProtocol::default())
        }
    }

    pub fn change_protocol_version(&mut self, version: Box<dyn MessageProtocolApi + 'static>) {
        self.version = version;
    }

    pub async fn read_with_fixed_16bit_header(&mut self) -> Result<Vec<u8>, tokio::io::Error> {
        self.version.read_with_fixed_16bit_header(&mut self.inner).await
    }

    pub async fn read_with_fixed_32bit_header(&mut self) -> Result<Vec<u8>, tokio::io::Error> {
        self.version.read_with_fixed_32bit_header(&mut self.inner).await
    }

    pub async fn read_buf_with_header(&mut self, wire_encryption: &Option<EncryptKey>, total_read: &mut u64) -> Result<Vec<u8>, TError> {
        self.version.read_buf_with_header(&mut self.inner, wire_encryption, total_read).await
    }

    #[allow(dead_code)]
    pub fn protocol(&self) -> StreamProtocol {
        self.protocol.clone()
    }
}

#[async_trait::async_trait]
impl StreamReader
for StreamRx
{
    async fn read_buf_with_header(&mut self, wire_encryption: &Option<EncryptKey>, total_read: &mut u64) -> Result<Vec<u8>, TError> {
        StreamRx::read_buf_with_header(self, wire_encryption, total_read).await
    }
}

#[async_trait::async_trait]
impl StreamReceiver
for StreamRx
{
    async fn recv(&mut self, ek: &Option<EncryptKey>) -> Result<Vec<u8>, std::io::Error> {
        let mut total_read = 0u64;
        StreamRx::read_buf_with_header(self, ek, &mut total_read).await
    }
}

#[async_trait::async_trait]
impl StreamTransmitter
for StreamTx
{
    async fn send(&mut self, ek: &Option<EncryptKey>, data: &[u8]) -> Result<(), std::io::Error> {
        StreamTx::send(self, ek, data).await?;
        Ok(())
    }
}