//! Client State Machine
//!
//! Manages the game client state transitions.

use game_types::{GameState, MenuAction};

/// Client state with additional client-specific data
#[derive(Debug, Clone)]
pub struct ClientState {
    /// Current game state
    pub game_state: GameState,
    /// Frame count
    pub frame_count: u64,
    /// Matchmaking timer
    pub matchmaking_timer: f32,
    /// Lobby countdown timer
    pub countdown_timer: f32,
}

impl Default for ClientState {
    fn default() -> Self {
        Self {
            game_state: GameState::PartyLobby,
            frame_count: 0,
            matchmaking_timer: 0.0,
            countdown_timer: 10.0,
        }
    }
}

impl ClientState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update state based on menu action
    pub fn handle_menu_action(&mut self, action: MenuAction) -> Option<StateTransition> {
        match self.game_state {
            GameState::PartyLobby => match action {
                MenuAction::Select => Some(StateTransition::StartMatchmaking),
                MenuAction::Back => Some(StateTransition::OpenSettings),
                _ => None,
            },
            GameState::Settings => match action {
                MenuAction::Back => Some(StateTransition::BackToLobby),
                _ => None,
            },
            GameState::Matchmaking { .. } => match action {
                MenuAction::Back => Some(StateTransition::CancelMatchmaking),
                _ => None,
            },
            _ => None,
        }
    }

    /// Apply a state transition
    pub fn apply_transition(&mut self, transition: StateTransition) {
        match transition {
            StateTransition::StartMatchmaking => {
                self.game_state = GameState::Matchmaking { elapsed_secs: 0 };
                self.matchmaking_timer = 0.0;
            }
            StateTransition::CancelMatchmaking => {
                self.game_state = GameState::PartyLobby;
            }
            StateTransition::MatchFound => {
                self.game_state = GameState::LobbyIsland;
            }
            StateTransition::StartCountdown => {
                self.game_state = GameState::LobbyCountdown { remaining_secs: 10 };
                self.countdown_timer = 10.0;
            }
            StateTransition::StartBus => {
                self.game_state = GameState::BusPhase;
            }
            StateTransition::StartGame => {
                self.game_state = GameState::InGame;
            }
            StateTransition::Victory(winner_id) => {
                self.game_state = GameState::Victory { winner_id };
            }
            StateTransition::BackToLobby => {
                self.game_state = GameState::PartyLobby;
            }
            StateTransition::OpenSettings => {
                self.game_state = GameState::Settings;
            }
            StateTransition::OpenCustomization => {
                self.game_state = GameState::Customization;
            }
            StateTransition::OpenTestMap => {
                self.game_state = GameState::TestMap;
            }
        }
    }

    /// Update timers
    pub fn update(&mut self, dt: f32) {
        self.frame_count += 1;

        match &mut self.game_state {
            GameState::Matchmaking { elapsed_secs } => {
                self.matchmaking_timer += dt;
                *elapsed_secs = self.matchmaking_timer as u16;
            }
            GameState::LobbyCountdown { remaining_secs } => {
                self.countdown_timer -= dt;
                *remaining_secs = self.countdown_timer.max(0.0) as u8;
                if self.countdown_timer <= 0.0 {
                    self.apply_transition(StateTransition::StartBus);
                }
            }
            _ => {}
        }
    }
}

/// State transitions
#[derive(Debug, Clone, Copy)]
pub enum StateTransition {
    StartMatchmaking,
    CancelMatchmaking,
    MatchFound,
    StartCountdown,
    StartBus,
    StartGame,
    Victory(Option<u8>),
    BackToLobby,
    OpenSettings,
    OpenCustomization,
    OpenTestMap,
}
