use crate::platform::network::{
    Control, Endpoint, NetworkInterface, Packet, PacketType, TcpListener, TcpStream,
};
use crate::platform_testing::network::SimulatedNetwork;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::spawn;

#[derive(Debug)]
pub struct SimulatedNetworkInterface {
    network: Arc<SimulatedNetwork>,
    ip_addresses: Arc<Mutex<HashSet<String>>>,
    allocated_ports: Arc<Mutex<HashMap<String, HashSet<u16>>>>,
    tcp_listeners: Arc<Mutex<HashMap<(String, u16), Arc<SimulatedTcpListener>>>>,
    streams: Arc<Mutex<HashMap<(String, u16), Arc<SimulatedTcpStream>>>>,
}

impl SimulatedNetworkInterface {
    pub fn new(network: Arc<SimulatedNetwork>) -> Self {
        SimulatedNetworkInterface {
            network,
            ip_addresses: Arc::new(Mutex::new(HashSet::new())),
            allocated_ports: Arc::new(Mutex::new(HashMap::new())),
            tcp_listeners: Arc::new(Mutex::new(HashMap::new())),
            streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_ip_addresses(&self) -> Vec<String> {
        self.ip_addresses
            .lock()
            .unwrap()
            .iter()
            .map(|ip| ip.to_string())
            .collect()
    }

    pub fn assign_ip_addresses(&self, ip_addresses: Vec<&str>) {
        let mut curr_ip_addresses = self.ip_addresses.lock().unwrap();
        for ip in ip_addresses {
            curr_ip_addresses.insert(ip.to_string());
        }
    }

    pub fn remove_ip_addresses(&self, ip_addresses: Vec<&str>) {
        let mut curr_ip_addresses = self.ip_addresses.lock().unwrap();
        for ip in ip_addresses {
            curr_ip_addresses.remove(ip);
        }
    }

    pub fn get_stream(
        &self,
        source_endpoint: &Endpoint,
        destination_endpoint: &Endpoint,
    ) -> Option<Arc<dyn TcpStream>> {
        let stream = self
            .streams
            .lock()
            .unwrap()
            .get(&(
                destination_endpoint.ip.to_string(),
                destination_endpoint.port,
            ))
            .map(|stream| Arc::clone(stream));
        match stream {
            Some(stream) => return Some(stream),
            None => {}
        }

        let listener = self
            .tcp_listeners
            .lock()
            .unwrap()
            .get(&(
                destination_endpoint.ip.to_string(),
                destination_endpoint.port,
            ))
            .map(|listener| Arc::clone(listener))?;
        listener
            .get_stream(&source_endpoint)
            .map(|stream| stream as Arc<dyn TcpStream>)
    }

    fn get_tcp_listener(&self, ip: &str, port: u16) -> Option<Arc<SimulatedTcpListener>> {
        self.tcp_listeners
            .lock()
            .unwrap()
            .get(&(ip.to_string(), port))
            .map(|tcp_listener| Arc::clone(tcp_listener))
    }

    pub fn pick_local_ip(&self, remote_ip: &str) -> Option<String> {
        self.ip_addresses
            .lock()
            .unwrap()
            .iter()
            .next()
            .map(|ip| ip.to_string())
    }

    fn allocate_port(&self, ip: &str) -> Option<u16> {
        let mut rng = rand::thread_rng();
        let mut allocated_ports = self.allocated_ports.lock().unwrap();
        let ports = allocated_ports
            .entry(ip.to_string())
            .or_insert(HashSet::new());
        if ports.len() == 65536 {
            return None;
        }

        loop {
            let port = rng.gen_range(0..65535);
            if !ports.contains(&port) {
                ports.insert(port);
                return Some(port);
            }
        }
    }

    fn free_port(&self, ip: &str, port: u16) {
        todo!()
    }
}

impl NetworkInterface for SimulatedNetworkInterface {
    fn connect(&self, remote_ip: &str, remote_port: u16) -> Option<Arc<dyn TcpStream>> {
        let local_ip = self.pick_local_ip(remote_ip)?;
        let local_port = self.allocate_port(local_ip.as_str())?;
        let remote_network_interface = self.network.get_network_interface(remote_ip)?;
        let remote_tcp_listener =
            remote_network_interface.get_tcp_listener(remote_ip, remote_port)?;
        let local_endpoint = Endpoint {
            ip: local_ip.clone(),
            port: local_port,
        };
        let remote_endpoint = Endpoint {
            ip: remote_ip.to_string(),
            port: remote_port,
        };

        let local_tcp_stream = Arc::new(SimulatedTcpStream::new(
            Arc::clone(&self.network),
            local_endpoint.clone(),
            remote_endpoint.clone(),
        ));
        let remote_tcp_stream =
            SimulatedTcpStream::new(Arc::clone(&self.network), remote_endpoint, local_endpoint);
        self.streams
            .lock()
            .unwrap()
            .insert((local_ip, local_port), Arc::clone(&local_tcp_stream));
        spawn(move || {
            remote_tcp_listener.on_new_connection(remote_tcp_stream);
        });
        loop {
            if let Some(control) = local_tcp_stream.receive_control() {
                println!(
                    "ClientStream received {:?} from {:?}",
                    control,
                    local_tcp_stream.get_remote_endpoint()
                );
                match control {
                    Control::Sync => {
                        break;
                    }
                }
            }
        }
        Some(local_tcp_stream)
    }

    fn bind_tcp(&self, local_ip: &str, local_port: u16) -> Option<Arc<dyn TcpListener>> {
        let mut allocated_ports = self.allocated_ports.lock().unwrap();
        allocated_ports
            .entry(local_ip.to_string())
            .or_insert(HashSet::new())
            .insert(local_port);

        let tcp_listener = Arc::new(SimulatedTcpListener::new());
        let mut tcp_listeners = self.tcp_listeners.lock().unwrap();

        let mut ips = Vec::new();
        if local_ip != "" {
            ips.push(local_ip.to_string());
        } else {
            let ip_addresses = self.ip_addresses.lock().unwrap();
            for ip in ip_addresses.iter() {
                ips.push(ip.to_string());
            }
        }

        for ip in ips {
            tcp_listeners.insert((ip.to_string(), local_port), Arc::clone(&tcp_listener));
        }

        Some(tcp_listener)
    }
}

#[derive(Debug)]
struct SimulatedTcpStream {
    network: Arc<SimulatedNetwork>,
    local_endpoint: Endpoint,
    remote_endpoint: Endpoint,
    data_sender: Sender<Box<[u8]>>,
    data_receiver: Arc<Mutex<Receiver<Box<[u8]>>>>,
    control_sender: Sender<Control>,
    control_receiver: Arc<Mutex<Receiver<Control>>>,
}

impl SimulatedTcpStream {
    fn new(
        network: Arc<SimulatedNetwork>,
        local_endpoint: Endpoint,
        remote_endpoint: Endpoint,
    ) -> Self {
        let (data_sender, data_receiver) = mpsc::channel();
        let (control_sender, control_receiver) = mpsc::channel();
        SimulatedTcpStream {
            network,
            local_endpoint,
            remote_endpoint,
            data_sender,
            data_receiver: Arc::new(Mutex::new(data_receiver)),
            control_sender,
            control_receiver: Arc::new(Mutex::new(control_receiver)),
        }
    }

    fn receive_control(&self) -> Option<Control> {
        self.control_receiver.lock().unwrap().recv().ok()
    }
}

impl TcpStream for SimulatedTcpStream {
    fn send(&self, data: &[u8]) {
        self.network.send_packet(&Packet {
            source: self.local_endpoint.clone(),
            destination: self.remote_endpoint.clone(),
            packet_type: PacketType::Data,
            control: None,
            payload: Some(Box::from(data)),
        });
    }

    fn receive(&self) -> Option<Box<[u8]>> {
        self.data_receiver.lock().unwrap().recv().ok()
    }

    fn send_control(&self, control: Control) {
        self.network.send_packet(&Packet {
            source: self.local_endpoint.clone(),
            destination: self.remote_endpoint.clone(),
            packet_type: PacketType::Control,
            control: Some(control),
            payload: None,
        });
    }

    fn get_local_endpoint(&self) -> Endpoint {
        self.local_endpoint.clone()
    }

    fn get_remote_endpoint(&self) -> Endpoint {
        self.remote_endpoint.clone()
    }

    fn on_packet_received(&self, packet: &Packet) {
        match &(packet.packet_type) {
            PacketType::Data => {
                let data = packet.payload.clone().unwrap();
                self.data_sender.send(data).unwrap();
            }
            PacketType::Control => {
                let control = packet.control.clone().unwrap();
                self.control_sender.send(control).unwrap();
            }
        }
    }
}

#[derive(Debug)]
struct SimulatedTcpListener {
    connections_sender: Sender<SimulatedTcpStream>,
    connections_receiver: Mutex<Receiver<SimulatedTcpStream>>,
    connections: Mutex<HashMap<(String, u16), Arc<SimulatedTcpStream>>>,
}

impl SimulatedTcpListener {
    fn new() -> Self {
        let (connections_sender, connections_receiver) = mpsc::channel();
        SimulatedTcpListener {
            connections_sender,
            connections_receiver: Mutex::new(connections_receiver),
            connections: Mutex::new(HashMap::new()),
        }
    }

    pub fn on_new_connection(&self, tcp_stream: SimulatedTcpStream) {
        self.connections_sender.send(tcp_stream).unwrap();
    }

    fn get_stream(&self, remote_endpoint: &Endpoint) -> Option<Arc<SimulatedTcpStream>> {
        self.connections
            .lock()
            .unwrap()
            .get(&(remote_endpoint.ip.clone(), remote_endpoint.port))
            .map(|tcp_stream| Arc::clone(tcp_stream))
    }
}

impl TcpListener for SimulatedTcpListener {
    fn accept(&self) -> Option<Arc<dyn TcpStream>> {
        let tcp_stream = self.connections_receiver.lock().unwrap().recv().ok()?;
        let tcp_stream = Arc::new(tcp_stream);
        let remote_endpoint = &tcp_stream.remote_endpoint;
        self.connections.lock().unwrap().insert(
            (remote_endpoint.ip.clone(), remote_endpoint.port),
            Arc::clone(&tcp_stream),
        );
        tcp_stream.send_control(Control::Sync);
        println!(
            "TcpListener sent {:?} to {:?}",
            Control::Sync,
            tcp_stream.get_remote_endpoint()
        );
        Some(tcp_stream)
    }
}
