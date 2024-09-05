use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;
use crate::platform::network::{Packet};
use crate::platform_testing::network_interface::SimulatedNetworkInterface;

#[derive(Clone, Debug)]
pub struct Endpoint {
    pub ip: String,
    pub port: u16,
}

#[derive(Debug)]
pub struct SimulatedNetwork {
    network_interfaces: Arc<Mutex<HashMap<String, Arc<SimulatedNetworkInterface>>>>,
    streams: Arc<Mutex<HashMap<(String, u16), UnboundedSender<Packet>>>>,
    connections: Arc<Mutex<HashSet<(String, String)>>>,
    delay_rate: f32,
    drop_rate: f32
}

impl SimulatedNetwork {
    pub fn new(delay_rate: f32, drop_rate: f32) -> Self {
        SimulatedNetwork {
            network_interfaces: Arc::new(Mutex::new(HashMap::new())),
            streams: Arc::new(Mutex::new(HashMap::new())),
            connections: Arc::new(Mutex::new(HashSet::new())),
            delay_rate,
            drop_rate
        }
    }

    pub fn get_network_interface(&self, ip: &str) -> Option<Arc<SimulatedNetworkInterface>> {
        let network_interfaces = self.network_interfaces.lock().unwrap();
        network_interfaces.get(ip).map(|network_interface| network_interface.clone())
    }

    pub fn send_packet(&self, packet: &Packet) {
        let network_interfaces = self.network_interfaces.lock().unwrap();
        let destination_network_interface = match network_interfaces.get(&packet.destination.ip) {
            None => return,
            Some(network_interface) => network_interface
        };
        let stream = match destination_network_interface
            .get_stream(&packet.source, &packet.destination) {
            None => return,
            Some(stream) => stream
        };
        stream.on_packet_received(packet)
    }

    pub fn register_network_interface(&self, network_interface: Arc<SimulatedNetworkInterface>) {
        let mut network_interfaces = self.network_interfaces.lock().unwrap();
        for ip in network_interface.get_ip_addresses() {
            network_interfaces.insert(ip.to_string(), Arc::clone(&network_interface));
        }
    }

    // pub fn send_packet(&self, packet: Packet) {
    //     let connections = self.connections.lock().unwrap();
    //     let connection_key = (packet.source_ip.to_string(), packet.destination_ip.to_string());
    //     if !connections.contains(&connection_key) {
    //         println!("No connection between {}:{}, dropping packet", packet.source_port, packet.destination_ip);
    //         return;
    //     }
    //
    //     let mut rng = rand::thread_rng();
    //     let mut packet_chance: f32 = rng.gen();
    //     if packet_chance <= self.drop_rate {
    //         println!("Drop packet");
    //         return;
    //     }
    //
    //     packet_chance -= self.drop_rate;
    //
    //     let streams = self.streams.lock().unwrap();
    //     let destination = (packet.destination_ip.to_string(), packet.destination_port);
    //     match streams.get(&destination) {
    //         None => return,
    //         Some(stream) => {
    //             if packet_chance <= self.delay_rate {
    //                 let delay = rng.gen_range(20..200);
    //                 println!("Delay packet to {}:{} for {}ms", packet.destination_ip, packet.destination_port, delay);
    //                 let stream = stream.clone();
    //                 tokio::spawn(async move {
    //                     time::sleep(Duration::from_millis(delay)).await;
    //                     stream.send(packet).unwrap();
    //                 });
    //                 return;
    //             }
    //
    //             stream.send(packet).unwrap();
    //         }
    //     }
    // }
    //
    // pub fn register_stream(&self, ip: &str, port: u16, stream: UnboundedSender<Packet>) {
    //     self.streams
    //         .lock()
    //         .unwrap()
    //         .insert((ip.to_string(), port), stream);
    // }
    //
    // pub fn connect(&self, ip1: &str, ip2: &str) {
    //     let mut connections = self.connections.lock().unwrap();
    //     connections.insert((ip1.to_string(), ip2.to_string()));
    //     connections.insert((ip2.to_string(), ip1.to_string()));
    // }
    //
    // pub fn disconnect(&self, ip1: &str, ip2: &str) {
    //     let mut connections = self.connections.lock().unwrap();
    //     connections.remove(&(ip1.to_string(), ip2.to_string()));
    //     connections.remove(&(ip2.to_string(), ip1.to_string()));
    // }
}