//! Input API
//!
//! Provides keyboard and mouse input services for applications.

use super::types::{KernelError, KernelResult};

/// Input service for polling keyboard and mouse
pub struct InputService {
    initialized: bool,
}

impl InputService {
    /// Create a new input service
    pub fn new() -> KernelResult<Self> {
        Ok(Self { initialized: true })
    }

    /// Poll for input events (call once per frame)
    pub fn poll(&mut self) {
        crate::game::input::poll_keyboard();
    }

    /// Get current keyboard state
    pub fn keyboard_state(&self) -> KeyState {
        let state = crate::game::input::KEY_STATE.lock();
        KeyState {
            w: state.w,
            a: state.a,
            s: state.s,
            d: state.d,
            space: state.space,
            ctrl: state.ctrl,
            shift: state.shift,
            escape: state.escape,
            enter: state.enter,
            tab: state.tab,
            up: state.up,
            down: state.down,
            left: state.left,
            right: state.right,
            one: state.one,
            two: state.two,
            three: state.three,
            four: state.four,
            five: state.five,
            q: state.q,
            e: state.e,
            r: state.r,
            f: state.f,
            b: state.b,
            t: state.t,
        }
    }

    /// Get current mouse state
    pub fn mouse_state(&self) -> MouseState {
        let state = crate::game::input::MOUSE_STATE.lock();
        MouseState {
            x: state.x,
            y: state.y,
            delta_x: state.delta_x,
            delta_y: state.delta_y,
            left_button: state.left_button,
            right_button: state.right_button,
            middle_button: state.middle_button,
        }
    }

    /// Reset mouse deltas (call after reading them)
    pub fn reset_mouse_deltas(&mut self) {
        crate::game::input::reset_mouse_deltas();
    }

    /// Get menu action from current input state (with edge detection)
    pub fn get_menu_action(&self, current: &KeyState, previous: &KeyState) -> MenuAction {
        // Edge detection - only trigger on key press, not hold
        if current.w && !previous.w || current.up && !previous.up {
            return MenuAction::Up;
        }
        if current.s && !previous.s || current.down && !previous.down {
            return MenuAction::Down;
        }
        if current.a && !previous.a || current.left && !previous.left {
            return MenuAction::Left;
        }
        if current.d && !previous.d || current.right && !previous.right {
            return MenuAction::Right;
        }
        if current.enter && !previous.enter || current.space && !previous.space {
            return MenuAction::Select;
        }
        if current.escape && !previous.escape {
            return MenuAction::Back;
        }
        MenuAction::None
    }
}

impl Default for InputService {
    fn default() -> Self {
        Self { initialized: false }
    }
}

/// Keyboard state
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyState {
    pub w: bool,
    pub a: bool,
    pub s: bool,
    pub d: bool,
    pub space: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub escape: bool,
    pub enter: bool,
    pub tab: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub one: bool,
    pub two: bool,
    pub three: bool,
    pub four: bool,
    pub five: bool,
    pub q: bool,
    pub e: bool,
    pub r: bool,
    pub f: bool,
    pub b: bool,
    pub t: bool,
}

impl KeyState {
    /// Check if any movement key is pressed
    pub fn any_movement(&self) -> bool {
        self.w || self.a || self.s || self.d
    }

    /// Check if any navigation key is pressed
    pub fn any_navigation(&self) -> bool {
        self.up || self.down || self.left || self.right || self.enter || self.escape
    }
}

/// Mouse state
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub delta_x: i32,
    pub delta_y: i32,
    pub left_button: bool,
    pub right_button: bool,
    pub middle_button: bool,
}

impl MouseState {
    /// Check if any button is pressed
    pub fn any_button(&self) -> bool {
        self.left_button || self.right_button || self.middle_button
    }
}

/// Menu navigation action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuAction {
    #[default]
    None,
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
}

impl MenuAction {
    /// Check if this is a directional action
    pub fn is_directional(&self) -> bool {
        matches!(self, Self::Up | Self::Down | Self::Left | Self::Right)
    }
}
