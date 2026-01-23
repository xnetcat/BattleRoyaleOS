//! PCI bus enumeration

use x86_64::instructions::port::Port;

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

/// PCI device information
#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub slot: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub bar0: u32,
    pub bar1: u32,
    pub interrupt_line: u8,
}

impl PciDevice {
    /// Read a 32-bit value from PCI configuration space
    pub fn read_config(&self, offset: u8) -> u32 {
        pci_read(self.bus, self.slot, self.function, offset)
    }

    /// Write a 32-bit value to PCI configuration space
    pub fn write_config(&self, offset: u8, value: u32) {
        pci_write(self.bus, self.slot, self.function, offset, value);
    }

    /// Enable bus mastering for DMA
    pub fn enable_bus_master(&self) {
        let command = self.read_config(0x04);
        self.write_config(0x04, command | 0x04); // Set Bus Master bit
    }

    /// Enable memory space access
    pub fn enable_memory_space(&self) {
        let command = self.read_config(0x04);
        self.write_config(0x04, command | 0x02); // Set Memory Space bit
    }

    /// Get BAR0 as memory address (mask off type bits)
    pub fn bar0_address(&self) -> u64 {
        (self.bar0 & 0xFFFFFFF0) as u64
    }
}

/// Read from PCI configuration space
fn pci_read(bus: u8, slot: u8, function: u8, offset: u8) -> u32 {
    let address: u32 = ((bus as u32) << 16)
        | ((slot as u32) << 11)
        | ((function as u32) << 8)
        | ((offset as u32) & 0xFC)
        | 0x80000000;

    unsafe {
        let mut addr_port = Port::<u32>::new(PCI_CONFIG_ADDRESS);
        let mut data_port = Port::<u32>::new(PCI_CONFIG_DATA);
        addr_port.write(address);
        data_port.read()
    }
}

/// Write to PCI configuration space
fn pci_write(bus: u8, slot: u8, function: u8, offset: u8, value: u32) {
    let address: u32 = ((bus as u32) << 16)
        | ((slot as u32) << 11)
        | ((function as u32) << 8)
        | ((offset as u32) & 0xFC)
        | 0x80000000;

    unsafe {
        let mut addr_port = Port::<u32>::new(PCI_CONFIG_ADDRESS);
        let mut data_port = Port::<u32>::new(PCI_CONFIG_DATA);
        addr_port.write(address);
        data_port.write(value);
    }
}

/// Enumerate all PCI devices
pub fn enumerate() -> alloc::vec::Vec<PciDevice> {
    let mut devices = alloc::vec::Vec::new();

    for bus in 0..=255u8 {
        for slot in 0..32u8 {
            for function in 0..8u8 {
                let vendor_device = pci_read(bus, slot, function, 0x00);
                let vendor_id = (vendor_device & 0xFFFF) as u16;

                if vendor_id == 0xFFFF {
                    continue;
                }

                let device_id = ((vendor_device >> 16) & 0xFFFF) as u16;
                let class_info = pci_read(bus, slot, function, 0x08);
                let class_code = ((class_info >> 24) & 0xFF) as u8;
                let subclass = ((class_info >> 16) & 0xFF) as u8;
                let bar0 = pci_read(bus, slot, function, 0x10);
                let bar1 = pci_read(bus, slot, function, 0x14);
                let interrupt_info = pci_read(bus, slot, function, 0x3C);
                let interrupt_line = (interrupt_info & 0xFF) as u8;

                devices.push(PciDevice {
                    bus,
                    slot,
                    function,
                    vendor_id,
                    device_id,
                    class_code,
                    subclass,
                    bar0,
                    bar1,
                    interrupt_line,
                });

                // If not multi-function device, skip remaining functions
                if function == 0 {
                    let header_type = pci_read(bus, slot, 0, 0x0C);
                    if (header_type >> 16) & 0x80 == 0 {
                        break;
                    }
                }
            }
        }
    }

    devices
}

/// Find a specific device by vendor and device ID
pub fn find_device(vendor_id: u16, device_id: u16) -> Option<PciDevice> {
    enumerate()
        .into_iter()
        .find(|d| d.vendor_id == vendor_id && d.device_id == device_id)
}

/// Intel E1000 vendor and device IDs
pub const INTEL_VENDOR_ID: u16 = 0x8086;
pub const E1000_DEVICE_ID: u16 = 0x100E;
