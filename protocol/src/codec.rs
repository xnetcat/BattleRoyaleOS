//! Binary serialization utilities

/// Write a u32 in little-endian format
pub fn write_u32(buf: &mut [u8], value: u32) {
    buf[0] = value as u8;
    buf[1] = (value >> 8) as u8;
    buf[2] = (value >> 16) as u8;
    buf[3] = (value >> 24) as u8;
}

/// Read a u32 in little-endian format
pub fn read_u32(buf: &[u8]) -> u32 {
    (buf[0] as u32)
        | ((buf[1] as u32) << 8)
        | ((buf[2] as u32) << 16)
        | ((buf[3] as u32) << 24)
}

/// Write an i32 in little-endian format
pub fn write_i32(buf: &mut [u8], value: i32) {
    write_u32(buf, value as u32);
}

/// Read an i32 in little-endian format
pub fn read_i32(buf: &[u8]) -> i32 {
    read_u32(buf) as i32
}

/// Write a u16 in little-endian format
pub fn write_u16(buf: &mut [u8], value: u16) {
    buf[0] = value as u8;
    buf[1] = (value >> 8) as u8;
}

/// Read a u16 in little-endian format
pub fn read_u16(buf: &[u8]) -> u16 {
    (buf[0] as u16) | ((buf[1] as u16) << 8)
}

/// Write an i16 in little-endian format
pub fn write_i16(buf: &mut [u8], value: i16) {
    write_u16(buf, value as u16);
}

/// Read an i16 in little-endian format
pub fn read_i16(buf: &[u8]) -> i16 {
    read_u16(buf) as i16
}
