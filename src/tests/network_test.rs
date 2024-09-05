#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread::{sleep, spawn};
    use std::time::Duration;
    use crate::platform::network::NetworkInterface;
    use crate::platform_testing::network::SimulatedNetwork;
    use crate::platform_testing::network_interface::SimulatedNetworkInterface;

    #[test]
    fn test_simulated_network_interface() {
        let network = Arc::new(SimulatedNetwork::new(0.1, 0.2));
        let network_interface1 = Arc::new(SimulatedNetworkInterface::new(Arc::clone(&network)));
        let network_interface2 = Arc::new(SimulatedNetworkInterface::new(Arc::clone(&network)));
        let network_interface3 = Arc::new(SimulatedNetworkInterface::new(Arc::clone(&network)));

        network_interface1.assign_ip_addresses(vec!["192.168.1.1"]);
        network_interface2.assign_ip_addresses(vec!["192.168.1.2", "192.168.1.4"]);
        network_interface3.assign_ip_addresses(vec!["192.168.1.3"]);

        network.register_network_interface(Arc::clone(&network_interface1));
        network.register_network_interface(Arc::clone(&network_interface2));
        network.register_network_interface(Arc::clone(&network_interface3));

        let server_port = 8080;
        let tcp_listener1 = match network_interface2.bind_tcp("", server_port) {
            None => return,
            Some(tcp_listener) => tcp_listener
        };
        println!("[Server] listening on {:?}", server_port);

        spawn(move || {
            let tcp_stream = match tcp_listener1.accept() {
                None => return,
                Some(tcp_stream) => tcp_stream
            };
            println!("[Server] accepted connection from {:?}", tcp_stream.get_remote_endpoint());
            spawn(move || {
                loop {
                    let data = match tcp_stream.receive() {
                        None => return,
                        Some(data) => data
                    };
                    let request = std::str::from_utf8(data.as_ref());
                    let request = match request {
                        Err(_) => return,
                        Ok(request) => request
                    };
                    println!("[Server] received from {:?}: {}", tcp_stream.get_remote_endpoint(), request);
                    let response = "World";
                    tcp_stream.send(response.as_bytes());
                    println!("[Server] replied to {:?}: {}", tcp_stream.get_remote_endpoint(), response);
                    return;
                }
            });
        });

        let server_ip = "192.168.1.2";
        println!("[Client] connecting to {:?}:{}", server_ip, server_port);
        let mut client_stream1 = network_interface1.connect(server_ip, server_port);
        assert!(!client_stream1.is_none());

        let client_stream1 = match client_stream1 {
            None => return,
            Some(tcp_stream) => tcp_stream
        };
        println!("[Client] connected to {:?}", client_stream1.get_remote_endpoint());
        let request = "Hello";

        println!("[Client] sending request to {:?}: {}", client_stream1.get_remote_endpoint(), request);
        client_stream1.send(request.as_bytes());
        println!("[Client] sent request to {:?}: {}", client_stream1.get_remote_endpoint(), request);

        println!("[Client] receiving response from {:?}", client_stream1.get_remote_endpoint());
        let data = client_stream1.receive();
        assert!(!data.is_none());
        let data = match data {
            None => return,
            Some(data) => data
        };
        println!("[Client] received response from {:?}: {}", client_stream1.get_remote_endpoint(), std::str::from_utf8(data.as_ref()).unwrap());
    }
}