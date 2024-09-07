use crate::platform::network::{Packet, TcpStream};
use crate::platform_testing::network_interface::SimulatedNetworkInterface;
use rand::rngs::ThreadRng;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug)]
pub struct SimulatedNetworkConfig {
    pub drop_rate: f32,
    pub short_delay_rate: f32,
    pub long_delay_rate: f32,
    pub short_delay_range: Range<Duration>,
    pub long_delay_range: Range<Duration>,
}

#[derive(Debug)]
pub struct SimulatedNetwork {
    network_interfaces: Arc<Mutex<HashMap<String, Arc<SimulatedNetworkInterface>>>>,
    connections: Arc<Mutex<HashSet<(String, String)>>>,
    config: SimulatedNetworkConfig,
}

impl SimulatedNetwork {
    pub fn new(config: SimulatedNetworkConfig) -> Self {
        SimulatedNetwork {
            network_interfaces: Arc::new(Mutex::new(HashMap::new())),
            connections: Arc::new(Mutex::new(HashSet::new())),
            config,
        }
    }

    pub fn get_network_interface(&self, ip: &str) -> Option<Arc<SimulatedNetworkInterface>> {
        let network_interfaces = self.network_interfaces.lock().unwrap();
        network_interfaces
            .get(ip)
            .map(|network_interface| network_interface.clone())
    }

    pub fn send_packet(&self, packet: &Packet) {
        let source_ip = packet.source.ip.to_string();
        let destination_ip = packet.destination.ip.to_string();
        let connections = self.connections.lock().unwrap();
        if !connections.contains(&(source_ip, destination_ip)) {
            println!(
                "No connection between {}:{}, dropping packet",
                packet.source.port, packet.destination.ip
            );
            return;
        }

        let mut rng = rand::thread_rng();
        let mut sample: f32 = rng.gen();
        if sample <= self.config.drop_rate {
            println!("Drop packet");
            return;
        }

        sample -= self.config.drop_rate;

        let network_interfaces = self.network_interfaces.lock().unwrap();
        let destination_network_interface = match network_interfaces.get(&packet.destination.ip) {
            None => return,
            Some(network_interface) => network_interface,
        };
        let stream =
            match destination_network_interface.get_stream(&packet.source, &packet.destination) {
                None => return,
                Some(stream) => stream,
            };

        if sample <= self.config.short_delay_rate {
            Self::delay_packet(
                &mut rng,
                stream,
                packet.clone(),
                self.config.short_delay_range.clone(),
            );
            return;
        }

        sample -= self.config.short_delay_rate;

        if sample <= self.config.long_delay_rate {
            Self::delay_packet(
                &mut rng,
                stream,
                packet.clone(),
                self.config.long_delay_range.clone(),
            );
            return;
        }

        sample -= self.config.long_delay_rate;
        stream.on_packet_received(packet)
    }

    pub fn connect(&self, ip1: &str, ip2: &str) {
        let mut connections = self.connections.lock().unwrap();
        connections.insert((ip1.to_string(), ip2.to_string()));
        connections.insert((ip2.to_string(), ip1.to_string()));
    }

    pub fn disconnect(&self, ip1: &str, ip2: &str) {
        let mut connections = self.connections.lock().unwrap();
        connections.remove(&(ip1.to_string(), ip2.to_string()));
        connections.remove(&(ip2.to_string(), ip1.to_string()));
    }

    pub fn register_network_interface(&self, network_interface: Arc<SimulatedNetworkInterface>) {
        let mut network_interfaces = self.network_interfaces.lock().unwrap();
        for ip in network_interface.get_ip_addresses() {
            network_interfaces.insert(ip.to_string(), Arc::clone(&network_interface));
        }
    }

    fn delay_packet(
        rng: &mut ThreadRng,
        stream: Arc<dyn TcpStream>,
        packet: Packet,
        delay_range: Range<Duration>,
    ) {
        let delay = rng.gen_range(delay_range);
        println!(
            "Delay packet to {}:{} for {}ms",
            packet.destination.ip,
            packet.destination.port,
            delay.as_millis()
        );
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            stream.on_packet_received(&packet);
        });
    }
}
