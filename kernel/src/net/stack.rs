//! Network stack wrapper using smoltcp

use super::device::E1000Device;
use crate::drivers::e1000::E1000_DEVICE;
use crate::serial_println;
use alloc::vec;
use smoltcp::iface::{Config, Interface, SocketHandle, SocketSet};
use smoltcp::socket::udp::{self, PacketBuffer, PacketMetadata};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, Ipv4Address};
use spin::Mutex;

/// Network stack state
pub struct NetworkStack {
    pub interface: Interface,
    pub device: E1000Device,
    pub sockets: SocketSet<'static>,
    pub udp_handle: Option<SocketHandle>,
}

impl NetworkStack {
    /// Create a new network stack
    pub fn new(mac: [u8; 6], ip: Ipv4Address) -> Self {
        let device = E1000Device::new();

        // Create interface config
        let config = Config::new(HardwareAddress::Ethernet(EthernetAddress(mac)));

        let mut interface = Interface::new(config, &mut E1000Device::new(), Instant::from_millis(0));

        // Set IP address
        interface.update_ip_addrs(|addrs| {
            addrs.push(IpCidr::new(IpAddress::Ipv4(ip), 24)).ok();
        });

        // Set default gateway (for QEMU user networking)
        interface
            .routes_mut()
            .add_default_ipv4_route(Ipv4Address::new(10, 0, 2, 2))
            .ok();

        // Create socket set
        let sockets = SocketSet::new(vec![]);

        Self {
            interface,
            device,
            sockets,
            udp_handle: None,
        }
    }

    /// Add a UDP socket for game protocol
    pub fn add_udp_socket(&mut self, port: u16) -> SocketHandle {
        // Create UDP socket buffers
        let rx_buffer = PacketBuffer::new(
            vec![PacketMetadata::EMPTY; 64],
            vec![0; 65535],
        );
        let tx_buffer = PacketBuffer::new(
            vec![PacketMetadata::EMPTY; 64],
            vec![0; 65535],
        );

        let mut socket = udp::Socket::new(rx_buffer, tx_buffer);
        socket.bind(port).expect("Failed to bind UDP socket");

        let handle = self.sockets.add(socket);
        self.udp_handle = Some(handle);

        serial_println!("NET: UDP socket bound to port {}", port);
        handle
    }

    /// Poll the network stack
    pub fn poll(&mut self, timestamp_ms: i64) {
        let timestamp = Instant::from_millis(timestamp_ms);
        self.interface
            .poll(timestamp, &mut self.device, &mut self.sockets);
    }

    /// Send a UDP packet
    pub fn send_udp(&mut self, dest_ip: Ipv4Address, dest_port: u16, data: &[u8]) -> bool {
        if let Some(handle) = self.udp_handle {
            let socket = self.sockets.get_mut::<udp::Socket>(handle);
            let endpoint = (IpAddress::Ipv4(dest_ip), dest_port);
            socket.send_slice(data, endpoint).is_ok()
        } else {
            false
        }
    }

    /// Receive a UDP packet
    pub fn recv_udp(&mut self) -> Option<(Ipv4Address, u16, alloc::vec::Vec<u8>)> {
        if let Some(handle) = self.udp_handle {
            let socket = self.sockets.get_mut::<udp::Socket>(handle);
            if socket.can_recv() {
                let mut buffer = vec![0u8; 2048];
                if let Ok((size, meta)) = socket.recv_slice(&mut buffer) {
                    buffer.truncate(size);
                    if let IpAddress::Ipv4(ip) = meta.endpoint.addr {
                        return Some((ip, meta.endpoint.port, buffer));
                    }
                }
            }
        }
        None
    }
}

/// Global network stack
pub static NETWORK_STACK: Mutex<Option<NetworkStack>> = Mutex::new(None);

/// Initialize the network stack
pub fn init() {
    let device_guard = E1000_DEVICE.lock();
    if let Some(device) = device_guard.as_ref() {
        let mac = device.mac_address();
        drop(device_guard);

        // Use 10.0.2.15 for QEMU user networking
        let ip = Ipv4Address::new(10, 0, 2, 15);

        let mut stack = NetworkStack::new(mac, ip);
        stack.add_udp_socket(5000); // Game protocol port

        *NETWORK_STACK.lock() = Some(stack);

        serial_println!("NET: Stack initialized with IP 10.0.2.15");
    }
}

/// Poll the network stack (call from main loop)
pub fn poll(timestamp_ms: i64) {
    if let Some(stack) = NETWORK_STACK.lock().as_mut() {
        stack.poll(timestamp_ms);
    }
}
