#![allow(unused_variables)]
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use ate::chain::ChainKey;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

mod common;

#[test]
fn test_udp_mesh() {
    common::run(async move {
        let _servers = common::setup().await;

        let chain = ChainKey::from("tokera.com/4932a508739386ec3c4d76d269fc30eb_edge");
        let access_token = "4d6d309c1e9c58d3b3493c0fd00554f1";
        
        let s1_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 2), 3000));
        let c1 = common::client1(s1_addr.ip().clone(), &chain, access_token).await;
        
        let s2_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 3), 3000));
        let c2 = common::client2(s2_addr.ip().clone(), &chain, access_token).await;
        
        let mut s1 = c1.bind_udp(s1_addr).await.unwrap();
        let mut s2 = c2.bind_udp(s2_addr).await.unwrap();
        
        tokio::task::spawn(async move {
            loop {
                let (test, peer) = s1.recv_from().await.unwrap();
                assert_eq!(test, vec![1,2,3]);
                s1.send_to(vec![4,5,6], peer).await.unwrap();    
            }
        });

        loop {
            s2.send_to(vec![1,2,3], s1_addr).await.unwrap();
            if let Ok(Some((test, addr))) = s2.try_recv_from() {
                assert_eq!(test, vec![4,5,6]);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
}