//! smoltcp Device trait implementation for E1000

use crate::drivers::e1000::{E1000, E1000_DEVICE, BUFFER_SIZE};
use smoltcp::phy::{self, Device, DeviceCapabilities, Medium};
use smoltcp::time::Instant;

/// E1000 device wrapper for smoltcp
pub struct E1000Device;

impl E1000Device {
    pub fn new() -> Self {
        Self
    }
}

impl Device for E1000Device {
    type RxToken<'a> = E1000RxToken;
    type TxToken<'a> = E1000TxToken;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let mut device_guard = E1000_DEVICE.lock();
        let device = device_guard.as_mut()?;

        if device.has_packet() {
            Some((E1000RxToken, E1000TxToken))
        } else {
            None
        }
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(E1000TxToken)
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.max_transmission_unit = 1500;
        caps.max_burst_size = Some(1);
        caps
    }
}

/// RX token for receiving packets
pub struct E1000RxToken;

impl phy::RxToken for E1000RxToken {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        let mut device_guard = E1000_DEVICE.lock();
        if let Some(device) = device_guard.as_mut() {
            if let Some(data) = device.receive() {
                return f(&data);
            }
        }
        // If no packet, return with empty slice
        f(&[])
    }
}

/// TX token for transmitting packets
pub struct E1000TxToken;

impl phy::TxToken for E1000TxToken {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buffer = [0u8; BUFFER_SIZE];
        let result = f(&mut buffer[..len]);

        let mut device_guard = E1000_DEVICE.lock();
        if let Some(device) = device_guard.as_mut() {
            let _ = device.transmit(&buffer[..len]);
        }

        result
    }
}
