//! Input handling

use protocol::packets::ClientInput;
use spin::Mutex;
use x86_64::instructions::port::Port;

/// PS/2 keyboard data port
const KEYBOARD_DATA_PORT: u16 = 0x60;
/// PS/2 keyboard status port
const KEYBOARD_STATUS_PORT: u16 = 0x64;

/// Key scan codes (Set 1)
pub mod ScanCode {
    pub const W: u8 = 0x11;
    pub const A: u8 = 0x1E;
    pub const S: u8 = 0x1F;
    pub const D: u8 = 0x20;
    pub const SPACE: u8 = 0x39;
    pub const LCTRL: u8 = 0x1D;
    pub const LSHIFT: u8 = 0x2A;
    pub const B: u8 = 0x30;
    pub const ESC: u8 = 0x01;
    pub const MOUSE_LEFT: u8 = 0xF0; // Synthetic
}

/// Key state
#[derive(Debug, Clone, Default)]
pub struct KeyState {
    pub w: bool,
    pub a: bool,
    pub s: bool,
    pub d: bool,
    pub space: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub b: bool,
    pub escape: bool,
    pub mouse_left: bool,
}

impl KeyState {
    /// Convert to client input
    pub fn to_input(&self, player_id: u8, sequence: u32, yaw: i16, pitch: i16) -> ClientInput {
        let forward = if self.w {
            1
        } else if self.s {
            -1
        } else {
            0
        };
        let strafe = if self.d {
            1
        } else if self.a {
            -1
        } else {
            0
        };

        ClientInput {
            player_id,
            sequence,
            forward,
            strafe,
            jump: self.space,
            crouch: self.ctrl,
            fire: self.mouse_left || self.shift,
            build: self.b,
            exit_bus: self.space, // Space also exits bus
            yaw,
            pitch,
        }
    }
}

/// Global key state
pub static KEY_STATE: Mutex<KeyState> = Mutex::new(KeyState {
    w: false,
    a: false,
    s: false,
    d: false,
    space: false,
    ctrl: false,
    shift: false,
    b: false,
    escape: false,
    mouse_left: false,
});

/// Poll keyboard (non-blocking)
pub fn poll_keyboard() {
    unsafe {
        let status = Port::<u8>::new(KEYBOARD_STATUS_PORT).read();

        // Check if there's data available
        if status & 0x01 == 0 {
            return;
        }

        let scancode = Port::<u8>::new(KEYBOARD_DATA_PORT).read();
        let released = scancode & 0x80 != 0;
        let code = scancode & 0x7F;

        let mut state = KEY_STATE.lock();

        match code {
            ScanCode::W => state.w = !released,
            ScanCode::A => state.a = !released,
            ScanCode::S => state.s = !released,
            ScanCode::D => state.d = !released,
            ScanCode::SPACE => state.space = !released,
            ScanCode::LCTRL => state.ctrl = !released,
            ScanCode::LSHIFT => state.shift = !released,
            ScanCode::B => state.b = !released,
            ScanCode::ESC => state.escape = !released,
            _ => {}
        }
    }
}

/// Get current input state
pub fn get_input(player_id: u8, sequence: u32, yaw: i16, pitch: i16) -> ClientInput {
    KEY_STATE.lock().to_input(player_id, sequence, yaw, pitch)
}

/// Check if escape is pressed
pub fn escape_pressed() -> bool {
    KEY_STATE.lock().escape
}
