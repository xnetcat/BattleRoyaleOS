//! Serial port (COM1) driver for debug output

use core::fmt::{self, Write};
use spin::Mutex;
use x86_64::instructions::port::Port;

const COM1_PORT: u16 = 0x3F8;

/// Global serial port instance
pub static SERIAL1: Mutex<SerialPort> = Mutex::new(SerialPort::new(COM1_PORT));

/// Serial port wrapper
pub struct SerialPort {
    data: Port<u8>,
    line_status: Port<u8>,
}

impl SerialPort {
    pub const fn new(base: u16) -> Self {
        Self {
            data: Port::new(base),
            line_status: Port::new(base + 5),
        }
    }

    /// Initialize the serial port
    pub fn init(&mut self) {
        unsafe {
            // Disable interrupts
            Port::<u8>::new(COM1_PORT + 1).write(0x00);
            // Enable DLAB (set baud rate divisor)
            Port::<u8>::new(COM1_PORT + 3).write(0x80);
            // Set divisor to 1 (lo byte) 115200 baud
            Port::<u8>::new(COM1_PORT + 0).write(0x01);
            // Hi byte
            Port::<u8>::new(COM1_PORT + 1).write(0x00);
            // 8 bits, no parity, one stop bit
            Port::<u8>::new(COM1_PORT + 3).write(0x03);
            // Enable FIFO, clear them, with 14-byte threshold
            Port::<u8>::new(COM1_PORT + 2).write(0xC7);
            // IRQs enabled, RTS/DSR set
            Port::<u8>::new(COM1_PORT + 4).write(0x0B);
        }
    }

    fn is_transmit_empty(&self) -> bool {
        unsafe { Port::<u8>::new(COM1_PORT + 5).read() & 0x20 != 0 }
    }

    /// Write a single byte to the serial port
    pub fn write_byte(&mut self, byte: u8) {
        while !self.is_transmit_empty() {
            core::hint::spin_loop();
        }
        unsafe {
            self.data.write(byte);
        }
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

/// Print to the serial port
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let _ = write!($crate::drivers::serial::SERIAL1.lock(), $($arg)*);
    }};
}

/// Print to the serial port with a newline
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => {{
        $crate::serial_print!($($arg)*);
        $crate::serial_print!("\n");
    }};
}
