//! Player Phase
//!
//! Defines the different phases a player can be in during a match.

/// Player's current phase within the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerPhase {
    /// On the battle bus, hasn't dropped yet
    #[default]
    OnBus,
    /// Freefalling after exiting bus
    Freefall,
    /// Glider/parachute deployed
    Gliding,
    /// On the ground, normal gameplay
    Grounded,
    /// Dead, eliminated from the match
    Eliminated,
    /// Spectating another player
    Spectating,
}

impl PlayerPhase {
    /// Whether the player can move in this phase
    pub fn can_move(&self) -> bool {
        matches!(self, Self::Freefall | Self::Gliding | Self::Grounded)
    }

    /// Whether the player can take damage in this phase
    pub fn can_take_damage(&self) -> bool {
        matches!(self, Self::Freefall | Self::Gliding | Self::Grounded)
    }

    /// Whether the player is in the air
    pub fn is_airborne(&self) -> bool {
        matches!(self, Self::OnBus | Self::Freefall | Self::Gliding)
    }

    /// Whether the player is alive
    pub fn is_alive(&self) -> bool {
        !matches!(self, Self::Eliminated)
    }

    /// Whether the player is playing (not spectating or eliminated)
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Eliminated | Self::Spectating)
    }
}
