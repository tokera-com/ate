use std::sync::Arc;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::future::Future;
use ate::prelude::*;
use atenet::opt::OptsNetworkServer;
use tokio::runtime::Builder;
use tokera::mio::Port;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

fn create_solo(ip: IpAddr, node_id: u32) -> OptsNetworkServer
{
    OptsNetworkServer {
        listen: ip,
        http_port: Some(8080),
        tls_port: Some(4443),
        udp_port: Some(2222),
        token_path: "~/ate/token".to_string(),
        db_url: url::Url::parse("ws://tokera.sh/db").unwrap(),
        auth_url: url::Url::parse("ws://tokera.sh/auth").unwrap(),
        inst_url: url::Url::parse("ws://tokera.sh/inst").unwrap(),
        node_id: Some(node_id),
        ttl: 300,
    }   
}

async fn create_node(ip: IpAddr, node_id: u32) -> Arc<ateweb::server::Server> {
    let mut conf = AteConfig::default();
    conf.record_type_name = true;

    let solo = create_solo(ip, node_id);
    
    let (server, _exit) = atenet::common::setup_server(
        solo,
        conf,
        None,
        None
    ).await.unwrap();

    server
}

pub fn run<F: Future>(future: F) -> F::Output {
    let runtime = Arc::new(Builder::new_current_thread().enable_all().build().unwrap());
    runtime.clone().block_on(future)
}

pub async fn setup() -> Vec<Arc<ateweb::server::Server>> {
    ate::log_init(1, false);
    //ate::log_init(3, false);

    let s1 = create_node(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 1).await;
    let s2 = create_node(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 3)), 2).await;

    vec![s1, s2]
}

pub async fn client1(chain: &ChainKey, access_token: &str, static_ip: Option<(IpAddr, IpAddr)>) -> Port
{
    let node = url::Url::parse("ws://127.0.0.2:8080/net").unwrap();
    client(node, chain.clone(), access_token.to_string(), static_ip).await
}

pub async fn client2(chain: &ChainKey, access_token: &str, static_ip: Option<(IpAddr, IpAddr)>) -> Port
{
    let node = url::Url::parse("ws://127.0.0.3:8080/net").unwrap();
    client(node, chain.clone(), access_token.to_string(), static_ip).await
}

pub async fn client(node: url::Url, chain: ChainKey, access_token: String, static_ip: Option<(IpAddr, IpAddr)>) -> Port
{
    let mut port = Port::new(node, chain, access_token).await.unwrap();
    if let Some(static_ip) = static_ip {
        port.add_ip(static_ip.0, 24).await.unwrap();
        port.add_default_route(static_ip.1).await.unwrap();
    } else {
        port.dhcp_acquire().await.unwrap();
    }
    port
}

pub async fn clients(cross_switch: bool, use_dhcp: bool) -> (Port, Port)
{
    let chain1 = ChainKey::from("tokera.com/c789bd1b7f2a9bba3111d707ce40bffa_edge");
    let access_token1 = "80f5bdf28ce21947795de68c93c57fcb";
    let addr1 = if use_dhcp == false {
        Some((IpAddr::V4(Ipv4Addr::new(10, 35, 196, 2)), IpAddr::V4(Ipv4Addr::new(10, 35, 196, 1))))
    } else {
        None
    };

    let (chain2, access_token2, addr2) = if cross_switch == true {
        let chain2 = ChainKey::from("tokera.com/d237f26dbb0c4cdc17b43d3720009e5d_edge");
        let access_token2 = "69aec1eaa6cb5a69acac91a8c38d7498";
        let addr2 = if use_dhcp == false {
            Some((IpAddr::V4(Ipv4Addr::new(10, 127, 26, 2)), IpAddr::V4(Ipv4Addr::new(10, 127, 26, 1))))
        } else {
            None
        };
        (chain2, access_token2, addr2)
    } else {
        let addr2 = if use_dhcp == false {
            Some((IpAddr::V4(Ipv4Addr::new(10, 35, 196, 3)), IpAddr::V4(Ipv4Addr::new(10, 35, 196, 1))))
        } else {
            None
        };
        (chain1.clone(), access_token1.clone(), addr2)
    };    

    let c1 = client1(&chain1, access_token1, addr1).await;
    let c2 = client2(&chain2, access_token2, addr2).await;

    (c1, c2)
}