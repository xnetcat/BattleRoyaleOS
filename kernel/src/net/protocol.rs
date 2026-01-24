//! Game network protocol handler

use super::stack::NETWORK_STACK;
use crate::game::world::GAME_WORLD;
use crate::serial_println;
use alloc::vec::Vec;
use alloc::string::String;
use protocol::packets::{ClientInput, Packet, PlayerState, WorldStateDelta};
use smoltcp::wire::Ipv4Address;

/// Game protocol port
pub const GAME_PORT: u16 = 5000;

/// Server tick rate (Hz)
pub const SERVER_TICK_RATE: u32 = 20;

/// Handle incoming game packets
pub fn process_incoming() {
    let mut stack_guard = NETWORK_STACK.lock();
    if let Some(stack) = stack_guard.as_mut() {
        while let Some((src_ip, src_port, data)) = stack.recv_udp() {
            if let Some(packet) = Packet::decode(&data) {
                handle_packet(src_ip, src_port, packet);
            }
        }
    }
}

/// Handle a decoded packet
fn handle_packet(src_ip: Ipv4Address, src_port: u16, packet: Packet) {
    match packet {
        Packet::ClientInput(input) => {
            // Update player state based on input
            if let Some(world) = GAME_WORLD.lock().as_mut() {
                world.apply_input(input.player_id, &input);
            }
        }
        Packet::JoinRequest { name } => {
            serial_println!("NET: Join request from {}:{} - {}", src_ip, src_port, name);
            // Assign player ID and send response
            if let Some(world) = GAME_WORLD.lock().as_mut() {
                if let Some(player_id) = world.add_player(&name, src_ip, src_port) {
                    send_join_response(src_ip, src_port, player_id);
                }
            }
        }
        Packet::JoinResponse { player_id } => {
            serial_println!("NET: Joined game with ID {}", player_id);
            if let Some(world) = GAME_WORLD.lock().as_mut() {
                world.local_player_id = Some(player_id);
            }
        }
        Packet::WorldStateDelta(delta) => {
            // Client received world update - apply interpolation
            if let Some(world) = GAME_WORLD.lock().as_mut() {
                if !world.is_server {
                    world.apply_delta(&delta);
                }
            }
        }
        Packet::Discovery => {
            if let Some(world) = GAME_WORLD.lock().as_ref() {
                if world.is_server {
                    let count = world.alive_count() as u8;
                    send_discovery_response(src_ip, src_port, "BattleRoyale Server", count);
                }
            }
        }
        Packet::DiscoveryResponse {
            server_name,
            player_count,
        } => {
            serial_println!(
                "NET: Found server '{}' with {} players at {}",
                server_name,
                player_count,
                src_ip
            );
            // TODO: Add to server list in UI
        }
        _ => {}
    }
}

/// Send join response to a new player
fn send_join_response(dest_ip: Ipv4Address, dest_port: u16, player_id: u8) {
    let packet = Packet::JoinResponse { player_id };
    let data = packet.encode();

    if let Some(stack) = NETWORK_STACK.lock().as_mut() {
        stack.send_udp(dest_ip, dest_port, &data);
    }
}

/// Send discovery response
fn send_discovery_response(dest_ip: Ipv4Address, dest_port: u16, name: &str, count: u8) {
    let packet = Packet::DiscoveryResponse {
        server_name: String::from(name),
        player_count: count,
    };
    let data = packet.encode();

    if let Some(stack) = NETWORK_STACK.lock().as_mut() {
        stack.send_udp(dest_ip, dest_port, &data);
    }
}

/// Broadcast discovery packet
pub fn broadcast_discovery() {
    let packet = Packet::Discovery;
    let data = packet.encode();

    if let Some(stack) = NETWORK_STACK.lock().as_mut() {
        // Broadcast to 255.255.255.255
        stack.send_udp(Ipv4Address::new(255, 255, 255, 255), GAME_PORT, &data);
    }
}

/// Broadcast world state delta to all connected clients
pub fn broadcast_world_state() {
    let world_guard = GAME_WORLD.lock();
    if let Some(world) = world_guard.as_ref() {
        let delta = world.get_delta();
        let packet = Packet::WorldStateDelta(delta);
        let data = packet.encode();

        drop(world_guard);

        // Get list of connected clients
        let clients: Vec<(Ipv4Address, u16)> = {
            let world_guard = GAME_WORLD.lock();
            if let Some(world) = world_guard.as_ref() {
                world
                    .players
                    .iter()
                    .filter(|p| p.connected)
                    .map(|p| (p.address, p.port))
                    .collect()
            } else {
                Vec::new()
            }
        };

        if let Some(stack) = NETWORK_STACK.lock().as_mut() {
            for (ip, port) in clients {
                stack.send_udp(ip, port, &data);
            }
        }
    }
}

/// Send client input to server
pub fn send_input(input: &ClientInput, server_ip: Ipv4Address) {
    let packet = Packet::ClientInput(input.clone());
    let data = packet.encode();

    if let Some(stack) = NETWORK_STACK.lock().as_mut() {
        stack.send_udp(server_ip, GAME_PORT, &data);
    }
}
