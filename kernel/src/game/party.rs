//! Party system for squad-based gameplay
//!
//! Manages parties of up to 4 players who can queue together.

use alloc::vec::Vec;
use spin::Mutex;
use super::state::PlayerCustomization;

/// Maximum party size
pub const MAX_PARTY_SIZE: usize = 4;

/// Party member status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyMemberStatus {
    /// Idle in party lobby
    Idle,
    /// Ready to queue
    Ready,
    /// Currently in matchmaking queue
    Queuing,
    /// On lobby island (warmup)
    InLobbyIsland,
    /// In active game
    InGame,
}

impl Default for PartyMemberStatus {
    fn default() -> Self {
        Self::Idle
    }
}

/// A party member
#[derive(Debug, Clone)]
pub struct PartyMember {
    /// Unique player ID
    pub player_id: u64,
    /// Player display name (up to 16 chars)
    pub name: [u8; 16],
    /// Is this the local player
    pub is_local: bool,
    /// Is this player the party leader
    pub is_leader: bool,
    /// Current status
    pub status: PartyMemberStatus,
    /// Player customization
    pub customization: PlayerCustomization,
}

impl PartyMember {
    /// Create a new party member
    pub fn new(player_id: u64, name: &str, is_local: bool, is_leader: bool) -> Self {
        let mut name_buf = [0u8; 16];
        let bytes = name.as_bytes();
        let len = bytes.len().min(16);
        name_buf[..len].copy_from_slice(&bytes[..len]);

        Self {
            player_id,
            name: name_buf,
            is_local,
            is_leader,
            status: PartyMemberStatus::Idle,
            customization: PlayerCustomization::default(),
        }
    }

    /// Get name as string slice
    pub fn name_str(&self) -> &str {
        let end = self.name.iter().position(|&b| b == 0).unwrap_or(16);
        core::str::from_utf8(&self.name[..end]).unwrap_or("???")
    }

    /// Check if this member is ready to start
    pub fn is_ready(&self) -> bool {
        matches!(self.status, PartyMemberStatus::Ready | PartyMemberStatus::Queuing)
    }
}

/// Game mode selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    /// Solo - 100 players, no teammates
    Solo,
    /// Duos - 50 teams of 2
    Duos,
    /// Squads - 25 teams of 4
    Squads,
}

impl GameMode {
    pub const COUNT: usize = 3;

    pub fn from_index(index: usize) -> Self {
        match index % Self::COUNT {
            0 => Self::Solo,
            1 => Self::Duos,
            _ => Self::Squads,
        }
    }

    pub fn to_index(self) -> usize {
        match self {
            Self::Solo => 0,
            Self::Duos => 1,
            Self::Squads => 2,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Solo => "SOLO",
            Self::Duos => "DUOS",
            Self::Squads => "SQUADS",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Solo => "100 players, no teammates",
            Self::Duos => "50 teams of 2",
            Self::Squads => "25 teams of 4",
        }
    }

    /// Maximum party size for this mode
    pub fn max_party_size(self) -> usize {
        match self {
            Self::Solo => 1,
            Self::Duos => 2,
            Self::Squads => 4,
        }
    }
}

impl Default for GameMode {
    fn default() -> Self {
        Self::Solo
    }
}

/// A party of players
#[derive(Debug, Clone)]
pub struct Party {
    /// Party members (up to 4)
    pub members: Vec<PartyMember>,
    /// Party code for invites (6 alphanumeric chars)
    pub party_code: [u8; 6],
    /// Selected game mode
    pub game_mode: GameMode,
    /// Is party currently in queue
    pub in_queue: bool,
    /// Queue start timestamp (for display)
    pub queue_start_time: u64,
}

impl Party {
    /// Create a new party with the local player as leader
    pub fn new(local_player_name: &str) -> Self {
        let mut members = Vec::new();
        members.push(PartyMember::new(0, local_player_name, true, true));

        Self {
            members,
            party_code: generate_party_code(0),
            game_mode: GameMode::Solo,
            in_queue: false,
            queue_start_time: 0,
        }
    }

    /// Get the party leader
    pub fn leader(&self) -> Option<&PartyMember> {
        self.members.iter().find(|m| m.is_leader)
    }

    /// Get the local player
    pub fn local_player(&self) -> Option<&PartyMember> {
        self.members.iter().find(|m| m.is_local)
    }

    /// Get mutable local player
    pub fn local_player_mut(&mut self) -> Option<&mut PartyMember> {
        self.members.iter_mut().find(|m| m.is_local)
    }

    /// Check if all members are ready
    pub fn all_ready(&self) -> bool {
        !self.members.is_empty() && self.members.iter().all(|m| m.is_ready())
    }

    /// Get ready count
    pub fn ready_count(&self) -> usize {
        self.members.iter().filter(|m| m.is_ready()).count()
    }

    /// Add a party member (returns false if party is full)
    pub fn add_member(&mut self, member: PartyMember) -> bool {
        if self.members.len() >= MAX_PARTY_SIZE {
            return false;
        }
        self.members.push(member);
        true
    }

    /// Remove a party member by ID
    pub fn remove_member(&mut self, player_id: u64) -> bool {
        if let Some(pos) = self.members.iter().position(|m| m.player_id == player_id) {
            let was_leader = self.members[pos].is_leader;
            self.members.remove(pos);

            // If leader left and there are other members, promote someone
            if was_leader && !self.members.is_empty() {
                self.members[0].is_leader = true;
            }
            true
        } else {
            false
        }
    }

    /// Set all members' status
    pub fn set_all_status(&mut self, status: PartyMemberStatus) {
        for member in &mut self.members {
            member.status = status;
        }
    }

    /// Start matchmaking queue
    pub fn start_queue(&mut self, timestamp: u64) -> bool {
        // Validate party size for game mode
        if self.members.len() > self.game_mode.max_party_size() {
            return false;
        }

        // All members must be ready
        if !self.all_ready() {
            return false;
        }

        self.in_queue = true;
        self.queue_start_time = timestamp;
        self.set_all_status(PartyMemberStatus::Queuing);
        true
    }

    /// Cancel matchmaking queue
    pub fn cancel_queue(&mut self) {
        self.in_queue = false;
        self.set_all_status(PartyMemberStatus::Ready);
    }

    /// Get party code as string
    pub fn code_str(&self) -> &str {
        core::str::from_utf8(&self.party_code).unwrap_or("??????")
    }
}

/// Generate a party code from a seed
fn generate_party_code(seed: u64) -> [u8; 6] {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut code = [0u8; 6];
    let mut s = seed;

    for c in &mut code {
        s = s.wrapping_mul(1103515245).wrapping_add(12345);
        *c = CHARS[(s >> 16) as usize % CHARS.len()];
    }

    code
}

/// Global party state
pub static PARTY: Mutex<Option<Party>> = Mutex::new(None);

/// Initialize party with local player
pub fn init_party(player_name: &str) {
    *PARTY.lock() = Some(Party::new(player_name));
}

/// Get a clone of the current party
pub fn get_party() -> Option<Party> {
    PARTY.lock().clone()
}

/// Toggle local player ready status
pub fn toggle_ready() -> bool {
    let mut party = PARTY.lock();
    if let Some(p) = party.as_mut() {
        if let Some(local) = p.local_player_mut() {
            local.status = match local.status {
                PartyMemberStatus::Idle => PartyMemberStatus::Ready,
                PartyMemberStatus::Ready => PartyMemberStatus::Idle,
                other => other,
            };
            return matches!(local.status, PartyMemberStatus::Ready);
        }
    }
    false
}

/// Set game mode
pub fn set_game_mode(mode: GameMode) {
    let mut party = PARTY.lock();
    if let Some(p) = party.as_mut() {
        p.game_mode = mode;
    }
}

/// Get current game mode
pub fn get_game_mode() -> GameMode {
    PARTY.lock().as_ref().map(|p| p.game_mode).unwrap_or_default()
}
