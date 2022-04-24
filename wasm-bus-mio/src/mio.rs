use std::sync::Arc;
use std::io;

pub use std::time::Duration;
pub use std::net::SocketAddr;
pub use std::net::Ipv4Addr;
pub use std::net::Ipv6Addr;

use super::api;

pub struct NetworkManagement
{
    wapm: String,
    #[allow(dead_code)]
    factory: api::MioClient,
}

impl NetworkManagement
{
    pub async fn new(wapm: &str) -> io::Result<NetworkManagement> {
        let factory = api::MioClient::new(wapm);
        Ok(
            NetworkManagement {
                wapm: wapm.to_string(),
                factory
            }
        )
    }

    pub async fn bind_raw(&self) -> io::Result<AsyncRawSocket> {
        AsyncRawSocket::bind(self.wapm.as_str()).await
    }

    pub async fn bind_tcp(&self, addr: SocketAddr) -> io::Result<AsyncTcpListener> {
        AsyncTcpListener::bind(self.wapm.as_str(), addr).await
    }

    pub async fn connect_tcp(&self, addr: SocketAddr, peer: SocketAddr) -> io::Result<AsyncTcpStream> {
        AsyncTcpStream::connect(self.wapm.as_str(), addr, peer).await
    }

    pub async fn bind_udp(&self, addr: SocketAddr) -> io::Result<AsyncUdpSocket> {
        AsyncUdpSocket::bind(self.wapm.as_str(), addr).await
    }

    pub fn blocking_bind_raw(&self) -> io::Result<RawSocket> {
        RawSocket::bind(self.wapm.as_str())
    }

    pub fn blocking_bind_tcp(&self, addr: SocketAddr) -> io::Result<TcpListener> {
        TcpListener::bind(self.wapm.as_str(), addr)
    }

    pub fn blocking_connect_tcp(&self, addr: SocketAddr, peer: SocketAddr) -> io::Result<TcpStream> {
        TcpStream::connect(self.wapm.as_str(), addr, peer)
    }

    pub fn blocking_bind_udp(&self, addr: SocketAddr) -> io::Result<UdpSocket> {
        UdpSocket::bind(self.wapm.as_str(), addr)
    }
}

pub struct AsyncRawSocket {
    raw: Arc<dyn api::RawSocket + Send + Sync + 'static>,
}

impl AsyncRawSocket {
    pub async fn bind(wapm: &str) -> io::Result<AsyncRawSocket> {
        let factory = api::MioClient::new(wapm);
        let raw = factory.bind_raw().await
            .map_err(|err| err.into_io_error())?;
        Ok(
            AsyncRawSocket {
                raw
            }
        )        
    }

    pub async fn send(&self, buf: Vec<u8>) -> io::Result<usize> {
        self.raw.send(buf).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn recv(&self, max: usize) -> io::Result<Vec<u8>> {
        self.raw.recv(max).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }
}

pub struct AsyncTcpListener {
    listener: Arc<dyn api::TcpListener + Send + Sync + 'static>,
}

impl AsyncTcpListener {
    pub async fn bind(wapm: &str, addr: SocketAddr) -> io::Result<AsyncTcpListener> {
        let factory = api::MioClient::new(wapm);
        let listener = factory
            .bind_tcp(addr)
            .await
            .map_err(conv_err)?;
        Ok(
            AsyncTcpListener {
                listener
            }
        )        
    }

    pub async fn accept(&self) -> io::Result<AsyncTcpStream> {
        let tcp = self.listener
            .accept()
            .await
            .map_err(conv_err)?;
        Ok(
            AsyncTcpStream {
                tcp
            }
        )
    }

    pub async fn listen(&self, backlog: u32) -> io::Result<()> {
        self.listener
            .listen(backlog)
            .await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(
            self.listener
                .local_addr()
                .await
                .map_err(conv_err)?
        )
    }

    pub async fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.listener
            .set_ttl(ttl)
            .await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn ttl(&self) -> io::Result<u32> {
        self.listener
            .ttl()
            .await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }
}

pub struct AsyncTcpStream {
    tcp: Arc<dyn api::TcpStream + Send + Sync + 'static>,
}

impl AsyncTcpStream {
    pub async fn connect(wapm: &str, addr: SocketAddr, peer: SocketAddr) -> io::Result<AsyncTcpStream> {
        let factory = api::MioClient::new(wapm);
        let tcp = factory.connect_tcp(addr, peer).await
            .map_err(conv_err)?;
        Ok(
            AsyncTcpStream {
                tcp
            }
        )
    }

    pub async fn peer_addr(&self) -> io::Result<SocketAddr> {
        Ok(
            self.tcp.peer_addr().await
                .map_err(conv_err)?
        )
    }

    pub async fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(
            self.tcp.local_addr().await
                .map_err(conv_err)?
        )
    }

    pub async fn shutdown(&self, shutdown: std::net::Shutdown) -> io::Result<()> {
        self.tcp.shutdown(shutdown.into()).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        self.tcp.set_nodelay(nodelay).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn nodelay(&self) -> io::Result<bool> {
        Ok(
            self.tcp.nodelay().await
                .map_err(conv_err)?
        )
    }

    pub async fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.tcp.set_ttl(ttl).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn ttl(&self) -> io::Result<u32> {
        Ok(
            self.tcp.ttl().await
                .map_err(conv_err)?
        )
    }

    pub async fn peek(&self, max: usize) -> io::Result<Vec<u8>> {
        self.tcp.peek(max).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn read(&self, max: usize) -> io::Result<Vec<u8>> {
        self.tcp.read(max).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn write(&self, buf: Vec<u8>) -> io::Result<usize> {
        self.tcp.write(buf).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn flush(&self) -> io::Result<()> {
        self.tcp.flush().await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn as_raw_fd(&self) -> io::Result<i32> {
        self.tcp.as_raw_fd().await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }
}

pub struct AsyncUdpSocket {
    udp: Arc<dyn api::UdpSocket + Send + Sync + 'static>,
}

impl AsyncUdpSocket {
    pub async fn bind(wapm: &str, addr: SocketAddr) -> io::Result<AsyncUdpSocket> {
        let factory = api::MioClient::new(wapm);
        let udp = factory.bind_udp(addr).await
            .map_err(|err| err.into_io_error())?;
        Ok(
            AsyncUdpSocket {
                udp
            }
        )        
    }

    pub async fn recv_from(&self, max: usize) -> io::Result<(Vec<u8>, SocketAddr)> {
        self.udp.recv_from(max).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn peek_from(&self, max: usize) -> io::Result<(Vec<u8>, SocketAddr)> {
        self.udp.peek_from(max).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn send_to(&self, buf: Vec<u8>, addr: SocketAddr) -> io::Result<usize> {
        self.udp.send_to(buf, addr).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn peer_addr(&self) -> io::Result<Option<SocketAddr>> {
        Ok(
            self.udp.peer_addr().await
                .map_err(conv_err)?
        )
    }

    pub async fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(
            self.udp.local_addr().await
                .map_err(conv_err)?
        )
    }

    pub async fn try_clone(&self) -> io::Result<AsyncUdpSocket> {
        let udp = self.udp.try_clone().await
            .map_err(conv_err)?;
        Ok(
            AsyncUdpSocket {
                udp
            }
        )
    }

    pub async fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.udp.set_read_timeout(dur).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.udp.set_write_timeout(dur).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn read_timeout(&self) -> io::Result<Option<Duration>> {
        Ok(
            self.udp.read_timeout().await
                .map_err(conv_err)?
        )
    }

    pub async fn write_timeout(&self) -> io::Result<Option<Duration>> {
        Ok(
            self.udp.write_timeout().await
                .map_err(conv_err)?
        )
    }

    pub async fn set_broadcast(&self, broadcast: bool) -> io::Result<()> {
        self.udp.set_broadcast(broadcast).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn broadcast(&self) -> io::Result<bool> {
        Ok(
            self.udp.broadcast().await
                .map_err(conv_err)?
        )
    }

    pub async fn set_multicast_loop_v4(&self, multicast_loop_v4: bool) -> io::Result<()> {
        self.udp.set_multicast_loop_v4(multicast_loop_v4).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn multicast_loop_v4(&self) -> io::Result<bool> {
        Ok(
            self.udp.multicast_loop_v4().await
                .map_err(conv_err)?
        )
    }

    pub async fn set_multicast_ttl_v4(&self, multicast_ttl_v4: u32) -> io::Result<()> {
        self.udp.set_multicast_ttl_v4(multicast_ttl_v4).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn multicast_ttl_v4(&self) -> io::Result<u32> {
        Ok(
            self.udp.multicast_ttl_v4().await
                .map_err(conv_err)?
        )
    }

    pub async fn set_multicast_loop_v6(&self, multicast_loop_v6: bool) -> io::Result<()> {
        self.udp.set_multicast_loop_v6(multicast_loop_v6).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn multicast_loop_v6(&self) -> io::Result<bool> {
        Ok(
            self.udp.multicast_loop_v6().await
                .map_err(conv_err)?
        )
    }

    pub async fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.udp.set_ttl(ttl).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn ttl(&self) -> io::Result<u32> {
        Ok(
            self.udp.ttl().await
                .map_err(conv_err)?
        )
    }

    pub async fn join_multicast_v4(&self, multiaddr: Ipv4Addr, interface: Ipv4Addr) -> io::Result<()> {
        self.udp.join_multicast_v4(multiaddr, interface).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn join_multicast_v6(&self, multiaddr: Ipv6Addr, interface: u32) -> io::Result<()> {
        self.udp.join_multicast_v6(multiaddr, interface).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn leave_multicast_v4(&self, multiaddr: Ipv4Addr, interface: Ipv4Addr) -> io::Result<()> {
        self.udp.leave_multicast_v4(multiaddr, interface).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn leave_multicast_v6(&self, multiaddr: Ipv6Addr, interface: u32) -> io::Result<()> {
        self.udp.leave_multicast_v6(multiaddr, interface).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn connect(&self, addr: SocketAddr) -> io::Result<()> {
        self.udp.connect(addr).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn send(&self, buf: Vec<u8>) -> io::Result<usize> {
        self.udp.send(buf).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn recv(&self, max: usize) -> io::Result<Vec<u8>> {
        self.udp.recv(max).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn peek(&self, max: usize) -> io::Result<Vec<u8>> {
        self.udp.peek(max).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.udp.set_nonblocking(nonblocking).await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub async fn as_raw_fd(&self) -> io::Result<i32> {
        self.udp.as_raw_fd().await
            .map_err(conv_err)?
            .map_err(conv_err2)
    }
}

pub struct RawSocket {
    raw: Arc<dyn api::RawSocket + Send + Sync + 'static>,
}

impl RawSocket {
    pub fn bind(wapm: &str) -> io::Result<RawSocket> {
        let factory = api::MioClient::new(wapm);
        let raw = factory.blocking_bind_raw()
            .map_err(|err| err.into_io_error())?;
        Ok(
            RawSocket {
                raw
            }
        )        
    }

    pub fn send(&self, buf: Vec<u8>) -> io::Result<usize> {
        self.raw.blocking_send(buf)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn recv(&self, max: usize) -> io::Result<Vec<u8>> {
        self.raw.blocking_recv(max)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }
}

pub struct TcpListener {
    listener: Arc<dyn api::TcpListener + Send + Sync + 'static>,
}

impl TcpListener {
    pub fn bind(wapm: &str, addr: SocketAddr) -> io::Result<TcpListener> {
        let factory = api::MioClient::new(wapm);
        let listener = factory.blocking_bind_tcp(addr)
            .map_err(conv_err)?;
        Ok(
            TcpListener {
                listener
            }
        )        
    }

    pub fn listen(&self, backlog: u32) -> io::Result<()> {
        self.listener.blocking_listen(backlog)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn accept(&self) -> io::Result<TcpStream> {
        let tcp = self.listener.blocking_accept()
            .map_err(conv_err)?;
        Ok(
            TcpStream {
                tcp
            }
        )
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(
            self.listener.blocking_local_addr()
                .map_err(conv_err)?
        )
    }

    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.listener.blocking_set_ttl(ttl)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn ttl(&self) -> io::Result<u32> {
        self.listener.blocking_ttl()
            .map_err(conv_err)?
            .map_err(conv_err2)
    }
}

pub struct TcpStream {
    tcp: Arc<dyn api::TcpStream + Send + Sync + 'static>,
}

impl TcpStream {
    pub fn connect(wapm: &str, addr: SocketAddr, peer: SocketAddr) -> io::Result<TcpStream> {
        let factory = api::MioClient::new(wapm);
        let tcp = factory.blocking_connect_tcp(addr, peer)
            .map_err(conv_err)?;
        Ok(
            TcpStream {
                tcp
            }
        )
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        Ok(
            self.tcp.blocking_peer_addr()
                .map_err(conv_err)?
        )
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(
            self.tcp.blocking_local_addr()
                .map_err(conv_err)?
        )
    }

    pub fn shutdown(&self, shutdown: std::net::Shutdown) -> io::Result<()> {
        self.tcp.blocking_shutdown(shutdown.into())
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        self.tcp.blocking_set_nodelay(nodelay)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn nodelay(&self) -> io::Result<bool> {
        Ok(
            self.tcp.blocking_nodelay()
                .map_err(conv_err)?
        )
    }

    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.tcp.blocking_set_ttl(ttl)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn ttl(&self) -> io::Result<u32> {
        Ok(
            self.tcp.blocking_ttl()
                .map_err(conv_err)?
        )
    }

    pub fn peek(&self, max: usize) -> io::Result<Vec<u8>> {
        self.tcp.blocking_peek(max)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn read(&self, max: usize) -> io::Result<Vec<u8>> {
        self.tcp.blocking_read(max)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn write(&self, buf: Vec<u8>) -> io::Result<usize> {
        self.tcp.blocking_write(buf)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn flush(&self) -> io::Result<()> {
        self.tcp.blocking_flush()
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn as_raw_fd(&self) -> io::Result<i32> {
        self.tcp.blocking_as_raw_fd()
            .map_err(conv_err)?
            .map_err(conv_err2)
    }
}

pub struct UdpSocket {
    udp: Arc<dyn api::UdpSocket + Send + Sync + 'static>,
}

impl UdpSocket {
    pub fn bind(wapm: &str, addr: SocketAddr) -> io::Result<UdpSocket> {
        let factory = api::MioClient::new(wapm);
        let udp = factory.blocking_bind_udp(addr)
            .map_err(|err| err.into_io_error())?;
        Ok(
            UdpSocket {
                udp
            }
        )        
    }

    pub fn recv_from(&self, max: usize) -> io::Result<(Vec<u8>, SocketAddr)> {
        self.udp.blocking_recv_from(max)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn peek_from(&self, max: usize) -> io::Result<(Vec<u8>, SocketAddr)> {
        self.udp.blocking_peek_from(max)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn send_to(&self, buf: Vec<u8>, addr: SocketAddr) -> io::Result<usize> {
        self.udp.blocking_send_to(buf, addr)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn peer_addr(&self) -> io::Result<Option<SocketAddr>> {
        Ok(
            self.udp.blocking_peer_addr()
                .map_err(conv_err)?
        )
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(
            self.udp.blocking_local_addr()
                .map_err(conv_err)?
        )
    }

    pub fn try_clone(&self) -> io::Result<UdpSocket> {
        let udp = self.udp.blocking_try_clone()
            .map_err(conv_err)?;
        Ok(
            UdpSocket {
                udp
            }
        )
    }

    pub fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.udp.blocking_set_read_timeout(dur)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.udp.blocking_set_write_timeout(dur)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
        Ok(
            self.udp.blocking_read_timeout()
                .map_err(conv_err)?
        )
    }

    pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
        Ok(
            self.udp.blocking_write_timeout()
                .map_err(conv_err)?
        )
    }

    pub fn set_broadcast(&self, broadcast: bool) -> io::Result<()> {
        self.udp.blocking_set_broadcast(broadcast)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn broadcast(&self) -> io::Result<bool> {
        Ok(
            self.udp.blocking_broadcast()
                .map_err(conv_err)?
        )
    }

    pub fn set_multicast_loop_v4(&self, multicast_loop_v4: bool) -> io::Result<()> {
        self.udp.blocking_set_multicast_loop_v4(multicast_loop_v4)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn multicast_loop_v4(&self) -> io::Result<bool> {
        Ok(
            self.udp.blocking_multicast_loop_v4()
                .map_err(conv_err)?
        )
    }

    pub fn set_multicast_ttl_v4(&self, multicast_ttl_v4: u32) -> io::Result<()> {
        self.udp.blocking_set_multicast_ttl_v4(multicast_ttl_v4)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn multicast_ttl_v4(&self) -> io::Result<u32> {
        Ok(
            self.udp.blocking_multicast_ttl_v4()
                .map_err(conv_err)?
        )
    }

    pub fn set_multicast_loop_v6(&self, multicast_loop_v6: bool) -> io::Result<()> {
        self.udp.blocking_set_multicast_loop_v6(multicast_loop_v6)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn multicast_loop_v6(&self) -> io::Result<bool> {
        Ok(
            self.udp.blocking_multicast_loop_v6()
                .map_err(conv_err)?
        )
    }

    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.udp.blocking_set_ttl(ttl)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn ttl(&self) -> io::Result<u32> {
        Ok(
            self.udp.blocking_ttl()
                .map_err(conv_err)?
        )
    }

    pub fn join_multicast_v4(&self, multiaddr: Ipv4Addr, interface: Ipv4Addr) -> io::Result<()> {
        self.udp.blocking_join_multicast_v4(multiaddr, interface)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn join_multicast_v6(&self, multiaddr: Ipv6Addr, interface: u32) -> io::Result<()> {
        self.udp.blocking_join_multicast_v6(multiaddr, interface)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn leave_multicast_v4(&self, multiaddr: Ipv4Addr, interface: Ipv4Addr) -> io::Result<()> {
        self.udp.blocking_leave_multicast_v4(multiaddr, interface)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn leave_multicast_v6(&self, multiaddr: Ipv6Addr, interface: u32) -> io::Result<()> {
        self.udp.blocking_leave_multicast_v6(multiaddr, interface)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn connect(&self, addr: SocketAddr) -> io::Result<()> {
        self.udp.blocking_connect(addr)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn send(&self, buf: Vec<u8>) -> io::Result<usize> {
        self.udp.blocking_send(buf)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn recv(&self, max: usize) -> io::Result<Vec<u8>> {
        self.udp.blocking_recv(max)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn peek(&self, max: usize) -> io::Result<Vec<u8>> {
        self.udp.blocking_peek(max)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.udp.blocking_set_nonblocking(nonblocking)
            .map_err(conv_err)?
            .map_err(conv_err2)
    }

    pub fn as_raw_fd(&self) -> io::Result<i32> {
        self.udp.blocking_as_raw_fd()
            .map_err(conv_err)?
            .map_err(conv_err2)
    }
}

fn conv_err(err: wasm_bus::abi::CallError) -> std::io::Error {
    err.into_io_error()
}

fn conv_err2(err: api::MioError) -> std::io::Error {
    err.into()
}