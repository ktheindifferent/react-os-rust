// HBA (Host Bus Adapter) Memory Management for AHCI

use crate::memory::PHYS_MEM_OFFSET;

// HBA Capabilities
pub fn parse_capabilities(cap: u32) -> HbaCapabilities {
    HbaCapabilities {
        num_ports: ((cap & 0x1F) + 1) as u8,
        num_cmd_slots: (((cap >> 8) & 0x1F) + 1) as u8,
        supports_64bit: (cap & (1 << 31)) != 0,
        supports_ncq: (cap & (1 << 30)) != 0,
        supports_snotif: (cap & (1 << 29)) != 0,
        supports_mps: (cap & (1 << 28)) != 0,
        supports_sss: (cap & (1 << 27)) != 0,
        supports_alpm: (cap & (1 << 26)) != 0,
        supports_al: (cap & (1 << 25)) != 0,
        supports_clo: (cap & (1 << 24)) != 0,
        interface_speed: ((cap >> 20) & 0xF) as u8,
        supports_ahci_only: (cap & (1 << 18)) != 0,
        supports_pm: (cap & (1 << 17)) != 0,
        supports_fbs: (cap & (1 << 16)) != 0,
        supports_pio_multiple: (cap & (1 << 15)) != 0,
        supports_slumber: (cap & (1 << 14)) != 0,
        supports_partial: (cap & (1 << 13)) != 0,
    }
}

#[derive(Debug, Clone)]
pub struct HbaCapabilities {
    pub num_ports: u8,
    pub num_cmd_slots: u8,
    pub supports_64bit: bool,
    pub supports_ncq: bool,
    pub supports_snotif: bool,
    pub supports_mps: bool,
    pub supports_sss: bool,
    pub supports_alpm: bool,
    pub supports_al: bool,
    pub supports_clo: bool,
    pub interface_speed: u8,
    pub supports_ahci_only: bool,
    pub supports_pm: bool,
    pub supports_fbs: bool,
    pub supports_pio_multiple: bool,
    pub supports_slumber: bool,
    pub supports_partial: bool,
}

impl HbaCapabilities {
    pub fn interface_speed_str(&self) -> &str {
        match self.interface_speed {
            1 => "1.5 Gbps",
            2 => "3 Gbps",
            3 => "6 Gbps",
            _ => "Unknown",
        }
    }
}