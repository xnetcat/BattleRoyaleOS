//! Boot Dispatcher
//!
//! Entry point logic for BattleRoyaleOS. Parses boot command line
//! and dispatches to the appropriate application mode.

#![no_std]

/// Application run mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Game client with full rendering
    GameClient,
    /// Dedicated server (headless)
    GameServer,
    /// Performance benchmark
    Benchmark,
    /// Test harness
    TestHarness,
}

impl Default for AppMode {
    fn default() -> Self {
        Self::GameClient
    }
}

impl AppMode {
    /// Parse from command line string
    pub fn from_cmdline(cmdline: &str) -> Self {
        let cmdline_lower = cmdline.to_lowercase_bytes();

        if contains_bytes(&cmdline_lower, b"server") {
            Self::GameServer
        } else if contains_bytes(&cmdline_lower, b"benchmark") {
            Self::Benchmark
        } else if contains_bytes(&cmdline_lower, b"test") {
            Self::TestHarness
        } else {
            Self::GameClient
        }
    }

    /// Whether this mode requires graphics
    pub fn needs_graphics(&self) -> bool {
        matches!(self, Self::GameClient | Self::Benchmark)
    }

    /// Whether this mode is headless (no rendering)
    pub fn is_headless(&self) -> bool {
        matches!(self, Self::GameServer | Self::TestHarness)
    }

    /// Get mode name
    pub fn name(&self) -> &'static str {
        match self {
            Self::GameClient => "Game Client",
            Self::GameServer => "Game Server",
            Self::Benchmark => "Benchmark",
            Self::TestHarness => "Test Harness",
        }
    }
}

/// Boot configuration parsed from command line
#[derive(Debug, Clone)]
pub struct BootConfig {
    pub mode: AppMode,
    pub debug: bool,
    pub server_port: u16,
    pub server_ip: Option<[u8; 4]>,
    pub benchmark_duration: u32,
    pub test_filter: Option<&'static str>,
}

impl Default for BootConfig {
    fn default() -> Self {
        Self {
            mode: AppMode::GameClient,
            debug: false,
            server_port: 5000,
            server_ip: None,
            benchmark_duration: 30,
            test_filter: None,
        }
    }
}

impl BootConfig {
    /// Parse boot configuration from command line
    pub fn from_cmdline(cmdline: &str) -> Self {
        let mut config = Self::default();

        config.mode = AppMode::from_cmdline(cmdline);

        // Check for debug flag
        if cmdline.contains("debug") {
            config.debug = true;
        }

        // Parse server port if specified (format: port=XXXX)
        if let Some(port_str) = find_value(cmdline, "port=") {
            if let Some(port) = parse_u16(port_str) {
                config.server_port = port;
            }
        }

        // Parse server IP if specified (format: ip=X.X.X.X)
        if let Some(ip_str) = find_value(cmdline, "ip=") {
            config.server_ip = parse_ip(ip_str);
        }

        // Parse benchmark duration (format: duration=XX)
        if let Some(dur_str) = find_value(cmdline, "duration=") {
            if let Some(dur) = parse_u32(dur_str) {
                config.benchmark_duration = dur;
            }
        }

        config
    }
}

/// Simple lowercase conversion for ASCII bytes
trait ToLowercaseBytes {
    fn to_lowercase_bytes(&self) -> [u8; 256];
}

impl ToLowercaseBytes for str {
    fn to_lowercase_bytes(&self) -> [u8; 256] {
        let mut result = [0u8; 256];
        for (i, b) in self.bytes().take(255).enumerate() {
            result[i] = if b >= b'A' && b <= b'Z' {
                b + 32
            } else {
                b
            };
        }
        result
    }
}

/// Check if byte slice contains pattern
fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    for i in 0..=(haystack.len() - needle.len()) {
        if &haystack[i..i + needle.len()] == needle {
            return true;
        }
    }
    false
}

/// Find value after a key in command line
fn find_value<'a>(cmdline: &'a str, key: &str) -> Option<&'a str> {
    if let Some(pos) = cmdline.find(key) {
        let start = pos + key.len();
        let remaining = &cmdline[start..];
        let end = remaining.find(' ').unwrap_or(remaining.len());
        Some(&remaining[..end])
    } else {
        None
    }
}

/// Parse u16 from string
fn parse_u16(s: &str) -> Option<u16> {
    let mut result: u16 = 0;
    for c in s.chars() {
        if c >= '0' && c <= '9' {
            result = result.checked_mul(10)?;
            result = result.checked_add((c as u16) - ('0' as u16))?;
        } else {
            break;
        }
    }
    if result > 0 { Some(result) } else { None }
}

/// Parse u32 from string
fn parse_u32(s: &str) -> Option<u32> {
    let mut result: u32 = 0;
    for c in s.chars() {
        if c >= '0' && c <= '9' {
            result = result.checked_mul(10)?;
            result = result.checked_add((c as u32) - ('0' as u32))?;
        } else {
            break;
        }
    }
    if result > 0 { Some(result) } else { None }
}

/// Parse IP address from string (X.X.X.X format)
fn parse_ip(s: &str) -> Option<[u8; 4]> {
    let mut parts = [0u8; 4];
    let mut part_idx = 0;
    let mut current: u16 = 0;
    let mut has_digit = false;

    for c in s.chars() {
        if c >= '0' && c <= '9' {
            current = current * 10 + (c as u16 - '0' as u16);
            has_digit = true;
            if current > 255 {
                return None;
            }
        } else if c == '.' {
            if !has_digit || part_idx >= 3 {
                return None;
            }
            parts[part_idx] = current as u8;
            part_idx += 1;
            current = 0;
            has_digit = false;
        } else {
            break;
        }
    }

    if has_digit && part_idx == 3 {
        parts[3] = current as u8;
        Some(parts)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_mode_parsing() {
        assert_eq!(AppMode::from_cmdline(""), AppMode::GameClient);
        assert_eq!(AppMode::from_cmdline("server"), AppMode::GameServer);
        assert_eq!(AppMode::from_cmdline("--mode=SERVER"), AppMode::GameServer);
        assert_eq!(AppMode::from_cmdline("benchmark"), AppMode::Benchmark);
        assert_eq!(AppMode::from_cmdline("test"), AppMode::TestHarness);
    }

    #[test]
    fn test_ip_parsing() {
        assert_eq!(parse_ip("10.0.2.15"), Some([10, 0, 2, 15]));
        assert_eq!(parse_ip("192.168.1.1"), Some([192, 168, 1, 1]));
        assert_eq!(parse_ip("invalid"), None);
        assert_eq!(parse_ip("256.0.0.1"), None);
    }
}
