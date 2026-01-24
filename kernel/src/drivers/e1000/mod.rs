//! Intel E1000 Network Driver

mod descriptors;
mod regs;
mod ring;

use crate::memory::dma::{phys_to_virt, virt_to_phys};
use crate::serial_println;
use alloc::vec::Vec;
use core::ptr::{read_volatile, write_volatile};
use spin::Mutex;

pub use descriptors::{RxDescriptor, TxDescriptor};
pub use regs::*;
pub use ring::{RxRing, TxRing};

/// Network device statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct DeviceStats {
    pub rx_packets: u64,
    pub rx_bytes: u64,
    pub tx_packets: u64,
    pub tx_bytes: u64,
}

/// Number of RX descriptors
pub const RX_RING_SIZE: usize = 256;
/// Number of TX descriptors
pub const TX_RING_SIZE: usize = 128;
/// Size of each packet buffer
pub const BUFFER_SIZE: usize = 2048;

/// E1000 Network Interface Controller
pub struct E1000 {
    mmio_base: u64,
    rx_ring: RxRing,
    tx_ring: TxRing,
    mac_address: [u8; 6],
    stats: DeviceStats,
}

impl E1000 {
    /// Create a new E1000 driver instance
    pub fn new(mmio_base: u64) -> Self {
        Self {
            mmio_base,
            rx_ring: RxRing::new(),
            tx_ring: TxRing::new(),
            mac_address: [0; 6],
            stats: DeviceStats::default(),
        }
    }

    /// Read from MMIO register
    fn read_reg(&self, reg: u32) -> u32 {
        unsafe {
            let ptr = (self.mmio_base + reg as u64) as *const u32;
            read_volatile(ptr)
        }
    }

    /// Write to MMIO register
    fn write_reg(&self, reg: u32, value: u32) {
        unsafe {
            let ptr = (self.mmio_base + reg as u64) as *mut u32;
            write_volatile(ptr, value);
        }
    }

    /// Initialize the E1000 device
    pub fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("E1000: Initializing at MMIO {:#x}", self.mmio_base);

        // Reset the device
        self.reset();

        // Read MAC address from EEPROM
        serial_println!("E1000: Reading MAC address...");
        self.read_mac_address();
        serial_println!(
            "E1000: MAC address: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.mac_address[0],
            self.mac_address[1],
            self.mac_address[2],
            self.mac_address[3],
            self.mac_address[4],
            self.mac_address[5]
        );

        // Initialize RX ring
        serial_println!("E1000: Initializing RX ring...");
        self.init_rx()?;
        serial_println!("E1000: RX ring initialized");

        // Initialize TX ring
        serial_println!("E1000: Initializing TX ring...");
        self.init_tx()?;
        serial_println!("E1000: TX ring initialized");

        // Enable RX interrupts (some E1000 implementations need this even for polling)
        self.write_reg(REG_IMC, 0xFFFFFFFF); // Clear all interrupt causes
        // Enable RX-related interrupts
        self.write_reg(REG_IMS, 0x000000FF); // Enable RX interrupts (RXDMT0, RXO, RXT0, etc.)

        // Set link up
        let ctrl = self.read_reg(REG_CTRL);
        self.write_reg(REG_CTRL, ctrl | CTRL_SLU);

        // Wait for link
        for _ in 0..100 {
            let status = self.read_reg(REG_STATUS);
            if status & STATUS_LU != 0 {
                serial_println!("E1000: Link up!");
                return Ok(());
            }
            // Small delay
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }

        serial_println!("E1000: Warning - link not detected, continuing anyway");
        Ok(())
    }

    /// Reset the device
    fn reset(&mut self) {
        // Set the reset bit
        self.write_reg(REG_CTRL, CTRL_RST);

        // Wait for reset to complete (~10ms)
        for _ in 0..200000 {
            core::hint::spin_loop();
        }

        // Wait for reset bit to clear
        for _ in 0..1000 {
            let ctrl = self.read_reg(REG_CTRL);
            if ctrl & CTRL_RST == 0 {
                break;
            }
            for _ in 0..100 {
                core::hint::spin_loop();
            }
        }

        // Disable interrupts after reset
        self.write_reg(REG_IMC, 0xFFFFFFFF);
    }

    /// Read MAC address
    fn read_mac_address(&mut self) {
        // Try reading from RAL/RAH first (set by QEMU)
        let ral = self.read_reg(REG_RAL);
        let rah = self.read_reg(REG_RAH);

        self.mac_address[0] = (ral & 0xFF) as u8;
        self.mac_address[1] = ((ral >> 8) & 0xFF) as u8;
        self.mac_address[2] = ((ral >> 16) & 0xFF) as u8;
        self.mac_address[3] = ((ral >> 24) & 0xFF) as u8;
        self.mac_address[4] = (rah & 0xFF) as u8;
        self.mac_address[5] = ((rah >> 8) & 0xFF) as u8;
    }

    /// Initialize receive ring
    fn init_rx(&mut self) -> Result<(), &'static str> {
        self.rx_ring.init()?;

        // Disable receiver during setup
        self.write_reg(REG_RCTL, 0);

        // Set RX descriptor base address
        let rx_desc_phys = self.rx_ring.descriptor_phys_addr();
        self.write_reg(REG_RDBAL, rx_desc_phys as u32);
        self.write_reg(REG_RDBAH, (rx_desc_phys >> 32) as u32);

        // Set RX descriptor ring length
        let rdlen = (RX_RING_SIZE * core::mem::size_of::<RxDescriptor>()) as u32;
        self.write_reg(REG_RDLEN, rdlen);

        // Set head pointer to 0
        self.write_reg(REG_RDH, 0);

        // Configure RX control (but not enabled yet)
        let rctl = RCTL_SBP |          // Store bad packets
            RCTL_UPE |          // Unicast promiscuous
            RCTL_MPE |          // Multicast promiscuous
            RCTL_BAM |          // Accept broadcast
            RCTL_BSIZE_2048 |   // Buffer size 2048
            RCTL_SECRC;         // Strip CRC
        self.write_reg(REG_RCTL, rctl);

        // Set tail pointer - this makes descriptors available to hardware
        self.write_reg(REG_RDT, (RX_RING_SIZE - 1) as u32);

        // Now enable the receiver
        self.write_reg(REG_RCTL, rctl | RCTL_EN);

        serial_println!("E1000: RX ring initialized");
        Ok(())
    }

    /// Initialize transmit ring
    fn init_tx(&mut self) -> Result<(), &'static str> {
        self.tx_ring.init()?;

        // Set TX descriptor base address
        let tx_desc_phys = self.tx_ring.descriptor_phys_addr();
        self.write_reg(REG_TDBAL, tx_desc_phys as u32);
        self.write_reg(REG_TDBAH, (tx_desc_phys >> 32) as u32);

        // Set TX descriptor ring length
        self.write_reg(
            REG_TDLEN,
            (TX_RING_SIZE * core::mem::size_of::<TxDescriptor>()) as u32,
        );

        // Set head and tail pointers
        self.write_reg(REG_TDH, 0);
        self.write_reg(REG_TDT, 0);

        // Configure TX control
        self.write_reg(
            REG_TCTL,
            TCTL_EN |           // Enable transmitter
            TCTL_PSP |          // Pad short packets
            (15 << TCTL_CT_SHIFT) |   // Collision threshold
            (64 << TCTL_COLD_SHIFT), // Collision distance
        );

        // Set inter-packet gap
        self.write_reg(REG_TIPG, 10 | (10 << 10) | (10 << 20));

        serial_println!("E1000: TX ring initialized");
        Ok(())
    }

    /// Get MAC address
    pub fn mac_address(&self) -> [u8; 6] {
        self.mac_address
    }

    /// Transmit a packet
    pub fn transmit(&mut self, data: &[u8]) -> Result<(), &'static str> {
        if data.len() > BUFFER_SIZE {
            return Err("Packet too large");
        }

        let tail = self.read_reg(REG_TDT) as usize;
        let desc = self.tx_ring.get_descriptor(tail);

        // Wait for descriptor to be available
        unsafe {
            while (*desc).status & TX_STATUS_DD == 0 {
                // Check if this is an uninitialized descriptor
                if (*desc).buffer_addr == 0 {
                    break;
                }
                core::hint::spin_loop();
            }
        }

        // Copy data to buffer and update descriptor
        self.tx_ring.prepare_send(tail, data);

        // Update tail pointer
        let new_tail = (tail + 1) % TX_RING_SIZE;
        self.write_reg(REG_TDT, new_tail as u32);

        // Update stats
        self.stats.tx_packets += 1;
        self.stats.tx_bytes += data.len() as u64;

        Ok(())
    }

    /// Receive a packet (returns None if no packet available)
    pub fn receive(&mut self) -> Option<Vec<u8>> {
        let tail = (self.read_reg(REG_RDT) as usize + 1) % RX_RING_SIZE;
        let desc = self.rx_ring.get_descriptor(tail);

        unsafe {
            // Check if descriptor has a packet
            if (*desc).status & RX_STATUS_DD == 0 {
                return None;
            }

            let length = (*desc).length as usize;
            if length == 0 || length > BUFFER_SIZE {
                // Reset descriptor and move on
                (*desc).status = 0;
                self.write_reg(REG_RDT, tail as u32);
                return None;
            }

            // Copy packet data
            let data = self.rx_ring.read_packet(tail, length);

            // Reset descriptor for reuse
            (*desc).status = 0;

            // Update tail pointer
            self.write_reg(REG_RDT, tail as u32);

            // Update stats
            self.stats.rx_packets += 1;
            self.stats.rx_bytes += data.len() as u64;

            Some(data)
        }
    }

    /// Check if there's a packet ready to receive
    pub fn has_packet(&self) -> bool {
        let rdt = self.read_reg(REG_RDT) as usize;
        let next = (rdt + 1) % RX_RING_SIZE;
        let desc = self.rx_ring.get_descriptor(next);
        let status = unsafe { (*desc).status };
        status & RX_STATUS_DD != 0
    }

    /// Check link status
    pub fn link_status(&self) -> bool {
        let status = self.read_reg(REG_STATUS);
        status & STATUS_LU != 0
    }

    /// Get device statistics
    pub fn get_stats(&self) -> DeviceStats {
        self.stats
    }
}

/// Global E1000 instance
pub static E1000_DEVICE: Mutex<Option<E1000>> = Mutex::new(None);

/// Initialize the E1000 driver with the given MMIO base address
pub fn init(mmio_base: u64) -> Result<(), &'static str> {
    let mut device = E1000::new(mmio_base);
    device.init()?;
    *E1000_DEVICE.lock() = Some(device);
    Ok(())
}
