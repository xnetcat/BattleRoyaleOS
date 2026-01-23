//! Packet definitions for game protocol

use alloc::string::String;
use alloc::vec::Vec;

/// Player state (24 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerState {
    pub player_id: u8,
    pub x: i32,      // Fixed-point 16.16
    pub y: i32,      // Fixed-point 16.16
    pub z: i32,      // Fixed-point 16.16
    pub yaw: i16,    // Rotation in degrees * 100
    pub pitch: i16,  // Rotation in degrees * 100
    pub health: u8,
    pub weapon_id: u8,
    pub state: u8,   // PlayerStateFlags
    _padding: u8,
}

impl PlayerState {
    pub const SIZE: usize = 24;

    pub fn new(player_id: u8) -> Self {
        Self {
            player_id,
            x: 0,
            y: 0,
            z: 0,
            yaw: 0,
            pitch: 0,
            health: 100,
            weapon_id: 0,
            state: 0,
            _padding: 0,
        }
    }

    /// Convert to world coordinates (from fixed-point)
    pub fn world_x(&self) -> f32 {
        self.x as f32 / 65536.0
    }

    pub fn world_y(&self) -> f32 {
        self.y as f32 / 65536.0
    }

    pub fn world_z(&self) -> f32 {
        self.z as f32 / 65536.0
    }

    /// Set position from world coordinates
    pub fn set_position(&mut self, x: f32, y: f32, z: f32) {
        self.x = (x * 65536.0) as i32;
        self.y = (y * 65536.0) as i32;
        self.z = (z * 65536.0) as i32;
    }

    /// Get yaw in radians
    pub fn yaw_radians(&self) -> f32 {
        (self.yaw as f32 / 100.0).to_radians()
    }

    /// Get pitch in radians
    pub fn pitch_radians(&self) -> f32 {
        (self.pitch as f32 / 100.0).to_radians()
    }
}

/// Player state flags
pub mod PlayerStateFlags {
    pub const ALIVE: u8 = 1 << 0;
    pub const JUMPING: u8 = 1 << 1;
    pub const CROUCHING: u8 = 1 << 2;
    pub const BUILDING: u8 = 1 << 3;
    pub const IN_BUS: u8 = 1 << 4;
    pub const PARACHUTE: u8 = 1 << 5;
}

/// Client input packet
#[derive(Debug, Clone, Default)]
pub struct ClientInput {
    pub player_id: u8,
    pub sequence: u32,
    pub forward: i8,     // -1, 0, 1
    pub strafe: i8,      // -1, 0, 1
    pub jump: bool,
    pub crouch: bool,
    pub fire: bool,
    pub build: bool,
    pub exit_bus: bool,
    pub yaw: i16,
    pub pitch: i16,
}

impl ClientInput {
    pub const SIZE: usize = 16;

    pub fn encode(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0] = self.player_id;
        buf[1..5].copy_from_slice(&self.sequence.to_le_bytes());
        buf[5] = self.forward as u8;
        buf[6] = self.strafe as u8;
        buf[7] = (self.jump as u8)
            | ((self.crouch as u8) << 1)
            | ((self.fire as u8) << 2)
            | ((self.build as u8) << 3)
            | ((self.exit_bus as u8) << 4);
        buf[8..10].copy_from_slice(&self.yaw.to_le_bytes());
        buf[10..12].copy_from_slice(&self.pitch.to_le_bytes());
        buf
    }

    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            player_id: buf[0],
            sequence: u32::from_le_bytes([buf[1], buf[2], buf[3], buf[4]]),
            forward: buf[5] as i8,
            strafe: buf[6] as i8,
            jump: buf[7] & 1 != 0,
            crouch: buf[7] & 2 != 0,
            fire: buf[7] & 4 != 0,
            build: buf[7] & 8 != 0,
            exit_bus: buf[7] & 16 != 0,
            yaw: i16::from_le_bytes([buf[8], buf[9]]),
            pitch: i16::from_le_bytes([buf[10], buf[11]]),
        })
    }
}

/// World state delta (only changed players)
#[derive(Debug, Clone, Default)]
pub struct WorldStateDelta {
    pub tick: u32,
    pub player_count: u8,
    pub players: Vec<PlayerState>,
    pub storm_x: i32,
    pub storm_z: i32,
    pub storm_radius: u32,
}

impl WorldStateDelta {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(16 + self.players.len() * PlayerState::SIZE);

        buf.extend_from_slice(&self.tick.to_le_bytes());
        buf.push(self.player_count);
        buf.extend_from_slice(&self.storm_x.to_le_bytes());
        buf.extend_from_slice(&self.storm_z.to_le_bytes());
        buf.extend_from_slice(&self.storm_radius.to_le_bytes());

        for player in &self.players {
            let bytes: [u8; PlayerState::SIZE] =
                unsafe { core::mem::transmute_copy(player) };
            buf.extend_from_slice(&bytes);
        }

        buf
    }

    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < 17 {
            return None;
        }

        let tick = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let player_count = buf[4];
        let storm_x = i32::from_le_bytes([buf[5], buf[6], buf[7], buf[8]]);
        let storm_z = i32::from_le_bytes([buf[9], buf[10], buf[11], buf[12]]);
        let storm_radius = u32::from_le_bytes([buf[13], buf[14], buf[15], buf[16]]);

        let mut players = Vec::with_capacity(player_count as usize);
        let mut offset = 17;

        for _ in 0..player_count {
            if offset + PlayerState::SIZE > buf.len() {
                break;
            }
            let mut state = PlayerState::default();
            let bytes: &[u8; PlayerState::SIZE] =
                buf[offset..offset + PlayerState::SIZE].try_into().ok()?;
            state = unsafe { core::mem::transmute_copy(bytes) };
            players.push(state);
            offset += PlayerState::SIZE;
        }

        Some(Self {
            tick,
            player_count,
            players,
            storm_x,
            storm_z,
            storm_radius,
        })
    }
}

/// Packet types
#[derive(Debug, Clone)]
pub enum Packet {
    /// Client requests to join game
    JoinRequest { name: String },
    /// Server responds with player ID
    JoinResponse { player_id: u8 },
    /// Client sends input
    ClientInput(ClientInput),
    /// Server sends world state
    WorldStateDelta(WorldStateDelta),
    /// Ping/pong for latency measurement
    Ping { timestamp: u64 },
    Pong { timestamp: u64 },
}

impl Packet {
    const TYPE_JOIN_REQUEST: u8 = 1;
    const TYPE_JOIN_RESPONSE: u8 = 2;
    const TYPE_CLIENT_INPUT: u8 = 3;
    const TYPE_WORLD_DELTA: u8 = 4;
    const TYPE_PING: u8 = 5;
    const TYPE_PONG: u8 = 6;

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        match self {
            Packet::JoinRequest { name } => {
                buf.push(Self::TYPE_JOIN_REQUEST);
                buf.push(name.len() as u8);
                buf.extend_from_slice(name.as_bytes());
            }
            Packet::JoinResponse { player_id } => {
                buf.push(Self::TYPE_JOIN_RESPONSE);
                buf.push(*player_id);
            }
            Packet::ClientInput(input) => {
                buf.push(Self::TYPE_CLIENT_INPUT);
                buf.extend_from_slice(&input.encode());
            }
            Packet::WorldStateDelta(delta) => {
                buf.push(Self::TYPE_WORLD_DELTA);
                buf.extend(delta.encode());
            }
            Packet::Ping { timestamp } => {
                buf.push(Self::TYPE_PING);
                buf.extend_from_slice(&timestamp.to_le_bytes());
            }
            Packet::Pong { timestamp } => {
                buf.push(Self::TYPE_PONG);
                buf.extend_from_slice(&timestamp.to_le_bytes());
            }
        }

        buf
    }

    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.is_empty() {
            return None;
        }

        match buf[0] {
            Self::TYPE_JOIN_REQUEST => {
                if buf.len() < 2 {
                    return None;
                }
                let len = buf[1] as usize;
                if buf.len() < 2 + len {
                    return None;
                }
                let name = String::from_utf8_lossy(&buf[2..2 + len]).into_owned();
                Some(Packet::JoinRequest { name })
            }
            Self::TYPE_JOIN_RESPONSE => {
                if buf.len() < 2 {
                    return None;
                }
                Some(Packet::JoinResponse { player_id: buf[1] })
            }
            Self::TYPE_CLIENT_INPUT => {
                let input = ClientInput::decode(&buf[1..])?;
                Some(Packet::ClientInput(input))
            }
            Self::TYPE_WORLD_DELTA => {
                let delta = WorldStateDelta::decode(&buf[1..])?;
                Some(Packet::WorldStateDelta(delta))
            }
            Self::TYPE_PING => {
                if buf.len() < 9 {
                    return None;
                }
                let timestamp = u64::from_le_bytes([
                    buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8],
                ]);
                Some(Packet::Ping { timestamp })
            }
            Self::TYPE_PONG => {
                if buf.len() < 9 {
                    return None;
                }
                let timestamp = u64::from_le_bytes([
                    buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8],
                ]);
                Some(Packet::Pong { timestamp })
            }
            _ => None,
        }
    }
}
