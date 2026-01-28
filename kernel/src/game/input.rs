//! Input handling with PS/2 keyboard and mouse support

use protocol::packets::ClientInput;
use spin::Mutex;
use x86_64::instructions::port::Port;

/// PS/2 keyboard data port
const KEYBOARD_DATA_PORT: u16 = 0x60;
/// PS/2 keyboard status port
const KEYBOARD_STATUS_PORT: u16 = 0x64;
/// PS/2 command port (for mouse commands)
const KEYBOARD_COMMAND_PORT: u16 = 0x64;

/// Key scan codes (Set 1)
pub mod ScanCode {
    pub const ESC: u8 = 0x01;
    pub const ONE: u8 = 0x02;
    pub const TWO: u8 = 0x03;
    pub const THREE: u8 = 0x04;
    pub const FOUR: u8 = 0x05;
    pub const FIVE: u8 = 0x06;
    pub const Q: u8 = 0x10;
    pub const W: u8 = 0x11;
    pub const E: u8 = 0x12;
    pub const R: u8 = 0x13;
    pub const A: u8 = 0x1E;
    pub const S: u8 = 0x1F;
    pub const D: u8 = 0x20;
    pub const F: u8 = 0x21;
    pub const TAB: u8 = 0x0F;
    pub const SPACE: u8 = 0x39;
    pub const LCTRL: u8 = 0x1D;
    pub const LSHIFT: u8 = 0x2A;
    pub const B: u8 = 0x30;
    pub const T: u8 = 0x14;
    pub const ENTER: u8 = 0x1C;
    pub const BACKSPACE: u8 = 0x0E;

    // Extended scan codes (prefixed with 0xE0)
    pub const EXTENDED: u8 = 0xE0;
    pub const UP: u8 = 0x48;
    pub const DOWN: u8 = 0x50;
    pub const LEFT: u8 = 0x4B;
    pub const RIGHT: u8 = 0x4D;
}

/// Mouse state
#[derive(Debug, Clone, Default)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub delta_x: i32,
    pub delta_y: i32,
    pub left_button: bool,
    pub right_button: bool,
    pub middle_button: bool,
    pub initialized: bool,
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
    pub t: bool,
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
            fire: self.shift,
            build: self.b,
            exit_bus: self.space, // Space also exits bus
            yaw,
            pitch,
        }
    }

    /// Check if any navigation key is pressed (for menus)
    pub fn any_nav_pressed(&self) -> bool {
        self.up || self.down || self.left || self.right || self.enter || self.escape
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
    enter: false,
    tab: false,
    up: false,
    down: false,
    left: false,
    right: false,
    one: false,
    two: false,
    three: false,
    four: false,
    five: false,
    q: false,
    e: false,
    r: false,
    f: false,
    t: false,
});

/// Global mouse state
pub static MOUSE_STATE: Mutex<MouseState> = Mutex::new(MouseState {
    x: 640,  // Start in center
    y: 400,
    delta_x: 0,
    delta_y: 0,
    left_button: false,
    right_button: false,
    middle_button: false,
    initialized: false,
});

/// Previous key state for edge detection
pub static PREV_KEY_STATE: Mutex<KeyState> = Mutex::new(KeyState {
    w: false,
    a: false,
    s: false,
    d: false,
    space: false,
    ctrl: false,
    shift: false,
    b: false,
    escape: false,
    enter: false,
    tab: false,
    up: false,
    down: false,
    left: false,
    right: false,
    one: false,
    two: false,
    three: false,
    four: false,
    five: false,
    q: false,
    e: false,
    r: false,
    f: false,
    t: false,
});

/// Track if we're in an extended key sequence
static EXTENDED_KEY: Mutex<bool> = Mutex::new(false);

/// Mouse packet state
static MOUSE_PACKET_STATE: Mutex<u8> = Mutex::new(0);
static MOUSE_PACKET: Mutex<[u8; 3]> = Mutex::new([0; 3]);

/// Wait for PS/2 controller input buffer to be empty
fn wait_write() {
    let mut timeout = 100000;
    unsafe {
        while timeout > 0 {
            if Port::<u8>::new(KEYBOARD_STATUS_PORT).read() & 0x02 == 0 {
                return;
            }
            timeout -= 1;
        }
    }
}

/// Wait for PS/2 controller output buffer to have data
fn wait_read() -> bool {
    let mut timeout = 100000;
    unsafe {
        while timeout > 0 {
            if Port::<u8>::new(KEYBOARD_STATUS_PORT).read() & 0x01 != 0 {
                return true;
            }
            timeout -= 1;
        }
    }
    false
}

/// Send command to PS/2 controller
fn send_command(cmd: u8) {
    wait_write();
    unsafe {
        Port::<u8>::new(KEYBOARD_COMMAND_PORT).write(cmd);
    }
}

/// Send data to PS/2 controller
fn send_data(data: u8) {
    wait_write();
    unsafe {
        Port::<u8>::new(KEYBOARD_DATA_PORT).write(data);
    }
}

/// Read data from PS/2 controller
fn read_data() -> Option<u8> {
    if wait_read() {
        unsafe { Some(Port::<u8>::new(KEYBOARD_DATA_PORT).read()) }
    } else {
        None
    }
}

/// Initialize PS/2 mouse
pub fn init_mouse() {
    // Flush any pending data from the controller
    unsafe {
        for _ in 0..100 {
            if Port::<u8>::new(KEYBOARD_STATUS_PORT).read() & 0x01 != 0 {
                let _ = Port::<u8>::new(KEYBOARD_DATA_PORT).read();
            }
        }
    }

    // Enable auxiliary device (mouse)
    send_command(0xA8);

    // Enable interrupts for mouse
    send_command(0x20);  // Read controller config
    if let Some(config) = read_data() {
        send_command(0x60);  // Write controller config
        send_data(config | 0x02);  // Enable IRQ12
    }

    // Reset mouse to known state
    send_command(0xD4);  // Send to mouse
    send_data(0xFF);     // Reset
    read_data();         // ACK
    read_data();         // Self-test result (0xAA)
    read_data();         // Mouse ID (0x00)

    // Set mouse defaults
    send_command(0xD4);  // Send to mouse
    send_data(0xF6);     // Set defaults
    read_data();         // Wait for ACK

    // Set sample rate to 100 samples/sec for smoother movement
    send_command(0xD4);
    send_data(0xF3);     // Set sample rate
    read_data();         // ACK
    send_command(0xD4);
    send_data(100);      // 100 samples/sec
    read_data();         // ACK

    // Enable mouse data reporting
    send_command(0xD4);  // Send to mouse
    send_data(0xF4);     // Enable
    read_data();         // Wait for ACK

    // Reset packet state
    *MOUSE_PACKET_STATE.lock() = 0;
    *MOUSE_PACKET.lock() = [0; 3];

    *MOUSE_STATE.lock() = MouseState {
        x: 512,  // Center of 1024 width
        y: 384,  // Center of 768 height
        delta_x: 0,
        delta_y: 0,
        left_button: false,
        right_button: false,
        middle_button: false,
        initialized: true,
    };
}

/// Poll keyboard and mouse (non-blocking)
/// Call this multiple times per frame to process all pending input
pub fn poll_keyboard() {
    // Process up to 32 bytes of input per call to handle accumulated data
    for _ in 0..32 {
        unsafe {
            let status = Port::<u8>::new(KEYBOARD_STATUS_PORT).read();

            // Check if there's data available
            if status & 0x01 == 0 {
                return;
            }

            let data = Port::<u8>::new(KEYBOARD_DATA_PORT).read();

            // Check if this is mouse data (bit 5 set in status)
            if status & 0x20 != 0 {
                handle_mouse_data(data);
                continue;
            }

            // Handle keyboard data
            let mut extended = EXTENDED_KEY.lock();

            if data == ScanCode::EXTENDED {
                *extended = true;
                continue;
            }

            let released = data & 0x80 != 0;
            let code = data & 0x7F;
            let is_extended = *extended;
            *extended = false;

            drop(extended);

            let mut state = KEY_STATE.lock();

            if is_extended {
                // Extended key codes
                match code {
                    ScanCode::UP => state.up = !released,
                    ScanCode::DOWN => state.down = !released,
                    ScanCode::LEFT => state.left = !released,
                    ScanCode::RIGHT => state.right = !released,
                    _ => {}
                }
            } else {
                // Regular key codes
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
                    ScanCode::ENTER => state.enter = !released,
                    ScanCode::TAB => state.tab = !released,
                    ScanCode::ONE => state.one = !released,
                    ScanCode::TWO => state.two = !released,
                    ScanCode::THREE => state.three = !released,
                    ScanCode::FOUR => state.four = !released,
                    ScanCode::FIVE => state.five = !released,
                    ScanCode::Q => state.q = !released,
                    ScanCode::E => state.e = !released,
                    ScanCode::R => state.r = !released,
                    ScanCode::F => state.f = !released,
                    ScanCode::T => state.t = !released,
                    _ => {}
                }
            }
        }
    }
}

/// Handle mouse data packet
fn handle_mouse_data(data: u8) {
    let mut packet_state = MOUSE_PACKET_STATE.lock();
    let mut packet = MOUSE_PACKET.lock();

    // Packet synchronization: first byte must have bit 3 set (always 1 in PS/2 mouse)
    // If we're expecting byte 0 and bit 3 is not set, this is a mid-packet byte - skip it
    if *packet_state == 0 {
        if data & 0x08 == 0 {
            // Invalid first byte, skip to resync
            return;
        }
    }

    packet[*packet_state as usize] = data;
    *packet_state += 1;

    if *packet_state >= 3 {
        *packet_state = 0;

        // Parse mouse packet
        let status = packet[0];
        let dx_raw = packet[1];
        let dy_raw = packet[2];

        // Check for overflow (discard packet)
        if status & 0xC0 != 0 {
            return;
        }

        // Calculate delta with proper sign extension
        // Bit 4 of status = X sign, Bit 5 = Y sign
        let delta_x = if status & 0x10 != 0 {
            dx_raw as i32 - 256  // Negative
        } else {
            dx_raw as i32       // Positive
        };

        let delta_y = if status & 0x20 != 0 {
            dy_raw as i32 - 256  // Negative
        } else {
            dy_raw as i32       // Positive
        };

        let mut mouse = MOUSE_STATE.lock();

        // Accumulate deltas (will be consumed and reset by game loop)
        mouse.delta_x += delta_x;
        mouse.delta_y += -delta_y;  // Invert Y for screen coordinates

        // Update absolute position for cursor (clamped to screen bounds)
        mouse.x = (mouse.x + delta_x).clamp(0, 1024);
        mouse.y = (mouse.y - delta_y).clamp(0, 768);

        // Update button states
        mouse.left_button = status & 0x01 != 0;
        mouse.right_button = status & 0x02 != 0;
        mouse.middle_button = status & 0x04 != 0;
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

/// Check if a key was just pressed (rising edge)
pub fn key_just_pressed(current: &KeyState, prev: &KeyState, check: fn(&KeyState) -> bool) -> bool {
    check(current) && !check(prev)
}

/// Save current key state as previous (call at end of frame)
pub fn save_key_state() {
    let current = KEY_STATE.lock().clone();
    *PREV_KEY_STATE.lock() = current;
}

/// Get previous key state
pub fn get_prev_key_state() -> KeyState {
    PREV_KEY_STATE.lock().clone()
}

/// Reset mouse deltas (call at end of frame after using them)
pub fn reset_mouse_deltas() {
    let mut mouse = MOUSE_STATE.lock();
    mouse.delta_x = 0;
    mouse.delta_y = 0;
}

/// Get mouse state
pub fn get_mouse_state() -> MouseState {
    MOUSE_STATE.lock().clone()
}

/// Menu input derived from key state
#[derive(Debug, Clone, Copy, Default)]
pub struct MenuInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub select: bool,
    pub back: bool,
}

impl MenuInput {
    /// Create from key state edge detection
    pub fn from_key_states(current: &KeyState, prev: &KeyState) -> Self {
        Self {
            up: current.up && !prev.up || current.w && !prev.w,
            down: current.down && !prev.down || current.s && !prev.s,
            left: current.left && !prev.left || current.a && !prev.a,
            right: current.right && !prev.right || current.d && !prev.d,
            select: current.enter && !prev.enter || current.space && !prev.space,
            back: current.escape && !prev.escape,
        }
    }

    /// Check if any input was given
    pub fn any(&self) -> bool {
        self.up || self.down || self.left || self.right || self.select || self.back
    }
}
