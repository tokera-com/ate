#![allow(unused_imports)]
use log::{info, warn, debug};
use crate::crypto::{EncryptKey, PrivateEncryptKey, PublicEncryptKey, InitializationVector};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use crate::prelude::*;
use super::MeshConfig;
use crate::comms::BroadcastContext;
#[cfg(all(feature = "enable_server", feature = "enable_tcp" ))]
use super::Listener;

#[cfg(test)]
#[derive(Serialize, Deserialize, Debug, Clone)]
enum TestMessage
{
    Noop,
    Rejected(Box<TestMessage>),
    Ping(String),
    Pong(String),
}

#[cfg(test)]
impl Default
for TestMessage
{
    fn default() -> TestMessage {
        TestMessage::Noop
    }
}

#[derive(Default)]
struct DummyContext {
}

impl BroadcastContext
for DummyContext {
    fn broadcast_group(&self) -> Option<u64> {
        None
    }
}

#[cfg(all(feature = "enable_server", feature = "enable_client", feature = "enable_tcp" ))]
#[tokio::main]
#[test]
async fn test_server_client_for_comms_with_tcp() -> Result<(), AteError> {
    test_server_client_for_comms(StreamProtocol::Tcp, 4001).await
}

#[cfg(all(feature = "enable_server", feature = "enable_client", feature = "enable_tcp" ))]
#[cfg(feature="enable_ws")]
#[tokio::main]
#[test]
async fn test_server_client_for_comms_with_websocket() -> Result<(), AteError> {
    test_server_client_for_comms(StreamProtocol::WebSocket, 4011).await
}

#[cfg(all(feature = "enable_server", feature = "enable_client", feature = "enable_tcp" ))]
#[cfg(test)]
async fn test_server_client_for_comms(wire_protocol: StreamProtocol, port: u16) -> Result<(), AteError> {
    crate::utils::bootstrap_env();
    
    let listener;
    let wire_format = SerializationFormat::MessagePack;
    {
        // Start the server
        info!("starting listen server on 127.0.0.1");

        let mut cfg = ConfMesh::for_domain("localhost".to_string());
        cfg.wire_protocol = wire_protocol;
        cfg.wire_format = wire_format;
        cfg.wire_encryption = Some(KeySize::Bit256);
        let cfg = MeshConfig::new(cfg)
            .listen_on(IpAddr::from_str("127.0.0.1")
            .unwrap(), port);
        
        listener = Listener::<TestMessage, DummyContext>::new(&cfg).await?;
        let (_, mut server_rx) = {
            let mut guard = listener.lock();
            guard.add_route("/comm-test")?
        };

        // Create a background thread that will respond to pings with pong
        info!("creating server worker thread");
        tokio::spawn(async move {
            while let Some(pck) = server_rx.recv().await {
                let data = pck.data;
                let pck: super::Packet<TestMessage> = pck.packet;
                match &pck.msg {
                    TestMessage::Ping(txt) => {
                        let _ = data.reply(TestMessage::Pong(txt.clone())).await;
                    },
                    _ => {}
                };
            }
        });
    }

    /* This has been disabled for now as we deprecated the built in relay functionality and will
     * build it again when the time is right
    {
        // Start the reply
        info!("start a client that will be relay server");
        let cfg = NodeConfig::new(wire_format)
            .wire_encryption(Some(KeySize::Bit256))
            .listen_on(IpAddr::from_str("127.0.0.1").unwrap(), port+1)
            .connect_to(IpAddr::from_str("127.0.0.1").unwrap(), port);
        let (relay_tx, mut relay_rx) = connect::<TestMessage, ()>(&cfg, None).await;

        // Create a background thread that will respond to pings with pong
        info!("start a client worker thread");
        tokio::spawn(async move {
            while let Some(pck) = relay_rx.recv().await {
                let data = pck.data;
                let pck = pck.packet;
                match pck.msg {
                    TestMessage::Ping(_) => relay_tx.upcast_packet(data).await.unwrap(),
                    TestMessage::Pong(_) => relay_tx.downcast_packet(data).await.unwrap(),
                    _ => data.reply(TestMessage::Rejected(Box::new(pck.msg.clone()))).await.unwrap(),
                };
            }
        });
    }
    */
    
    #[cfg(feature="enable_dns")]
    {
        // Start the client
        info!("start another client that will connect to the relay");
        
        let mut cfg = ConfMesh::for_domain("localhost".to_string());
        cfg.wire_protocol = wire_protocol;
        cfg.wire_format = wire_format;
        cfg.wire_encryption = Some(KeySize::Bit256);
        let cfg = MeshConfig::new(cfg)
            .connect_to(MeshAddress { host: IpAddr::from_str("127.0.0.1").unwrap(), port });
        let (client_tx, mut client_rx) = super::connect::<TestMessage, ()>(&cfg, "/comm-test".to_string())
            .await?;

        // We need to test it alot
        info!("send lots of hellos");
        for n in 0..1000
        {
            // Send a ping
            let test = format!("hello! {}", n);
            client_tx.send(TestMessage::Ping(test.clone()), None).await.unwrap();

            // Wait for the pong
            let pong = client_rx.recv().await.unwrap();
            let pong = pong.packet;
            if let TestMessage::Pong(txt) = pong.msg {
                assert_eq!(test, txt);
            } else {
                panic!("Wrong message type returned")
            }
        }
    }

    Ok(())
}