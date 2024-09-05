use std::fmt::Debug;
use std::sync::Arc;
use crate::platform_testing::network::Endpoint;

#[derive(Debug, Clone)]
pub enum Control {
    Sync,
}

#[derive(Debug)]
pub enum PacketType {
    Data,
    Control
}

#[derive(Debug)]
pub struct Packet {
    pub source: Endpoint,
    pub destination: Endpoint,
    pub packet_type: PacketType,
    pub control: Option<Control>,
    pub payload: Option<Box<[u8]>>,
}

pub trait NetworkInterface {
    fn connect(&self, remote_ip: &str, remote_port: u16) -> Option<Arc<dyn TcpStream>>;
    fn bind_tcp(&self, local_ip: &str, local_port: u16) -> Option<Arc<dyn TcpListener>>;
}

pub trait TcpStream : Debug + Send + Sync {
    fn send(&self, data: &[u8]);
    fn receive(&self) -> Option<Box<[u8]>>;
    fn send_control(&self, control: Control);
    fn get_local_endpoint(&self) -> Endpoint;
    fn get_remote_endpoint(&self) -> Endpoint;
    fn on_packet_received(&self, packet: &Packet);
}

pub trait TcpListener: Send + Sync {
    fn accept(&self) -> Option<Arc<dyn TcpStream>>;
}