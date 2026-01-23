//! E1000 Register Definitions

// Control registers
pub const REG_CTRL: u32 = 0x0000;
pub const REG_STATUS: u32 = 0x0008;
pub const REG_EECD: u32 = 0x0010;
pub const REG_EERD: u32 = 0x0014;

// Interrupt registers
pub const REG_ICR: u32 = 0x00C0;
pub const REG_ICS: u32 = 0x00C8;
pub const REG_IMS: u32 = 0x00D0;
pub const REG_IMC: u32 = 0x00D8;

// Receive registers
pub const REG_RCTL: u32 = 0x0100;
pub const REG_RDBAL: u32 = 0x2800;
pub const REG_RDBAH: u32 = 0x2804;
pub const REG_RDLEN: u32 = 0x2808;
pub const REG_RDH: u32 = 0x2810;
pub const REG_RDT: u32 = 0x2818;

// Transmit registers
pub const REG_TCTL: u32 = 0x0400;
pub const REG_TIPG: u32 = 0x0410;
pub const REG_TDBAL: u32 = 0x3800;
pub const REG_TDBAH: u32 = 0x3804;
pub const REG_TDLEN: u32 = 0x3808;
pub const REG_TDH: u32 = 0x3810;
pub const REG_TDT: u32 = 0x3818;

// MAC address registers
pub const REG_RAL: u32 = 0x5400;
pub const REG_RAH: u32 = 0x5404;

// Control register bits
pub const CTRL_SLU: u32 = 1 << 6; // Set Link Up
pub const CTRL_RST: u32 = 1 << 26; // Device Reset

// Status register bits
pub const STATUS_LU: u32 = 1 << 1; // Link Up

// Receive control bits
pub const RCTL_EN: u32 = 1 << 1; // Receiver Enable
pub const RCTL_SBP: u32 = 1 << 2; // Store Bad Packets
pub const RCTL_UPE: u32 = 1 << 3; // Unicast Promiscuous Enable
pub const RCTL_MPE: u32 = 1 << 4; // Multicast Promiscuous Enable
pub const RCTL_LPE: u32 = 1 << 5; // Long Packet Enable
pub const RCTL_BAM: u32 = 1 << 15; // Broadcast Accept Mode
pub const RCTL_BSIZE_2048: u32 = 0 << 16; // Buffer Size 2048
pub const RCTL_BSIZE_1024: u32 = 1 << 16; // Buffer Size 1024
pub const RCTL_BSIZE_512: u32 = 2 << 16; // Buffer Size 512
pub const RCTL_BSIZE_256: u32 = 3 << 16; // Buffer Size 256
pub const RCTL_SECRC: u32 = 1 << 26; // Strip Ethernet CRC

// Transmit control bits
pub const TCTL_EN: u32 = 1 << 1; // Transmitter Enable
pub const TCTL_PSP: u32 = 1 << 3; // Pad Short Packets
pub const TCTL_CT_SHIFT: u32 = 4; // Collision Threshold
pub const TCTL_COLD_SHIFT: u32 = 12; // Collision Distance

// TX descriptor command bits
pub const TX_CMD_EOP: u8 = 1 << 0; // End Of Packet
pub const TX_CMD_IFCS: u8 = 1 << 1; // Insert FCS
pub const TX_CMD_RS: u8 = 1 << 3; // Report Status

// TX descriptor status bits
pub const TX_STATUS_DD: u8 = 1 << 0; // Descriptor Done

// RX descriptor status bits
pub const RX_STATUS_DD: u8 = 1 << 0; // Descriptor Done
pub const RX_STATUS_EOP: u8 = 1 << 1; // End Of Packet
