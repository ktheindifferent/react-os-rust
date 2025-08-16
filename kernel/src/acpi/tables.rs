// ACPI Table Structures

// FADT (Fixed ACPI Description Table)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Fadt {
    pub header: super::SdtHeader,
    pub firmware_ctrl: u32,
    pub dsdt: u32,
    pub reserved: u8,
    pub preferred_pm_profile: u8,
    pub sci_interrupt: u16,
    pub smi_command_port: u32,
    pub acpi_enable: u8,
    pub acpi_disable: u8,
    pub s4bios_req: u8,
    pub pstate_control: u8,
    pub pm1a_event_block: u32,
    pub pm1b_event_block: u32,
    pub pm1a_control_block: u32,
    pub pm1b_control_block: u32,
    pub pm2_control_block: u32,
    pub pm_timer_block: u32,
    pub gpe0_block: u32,
    pub gpe1_block: u32,
    pub pm1_event_length: u8,
    pub pm1_control_length: u8,
    pub pm2_control_length: u8,
    pub pm_timer_length: u8,
    pub gpe0_block_length: u8,
    pub gpe1_block_length: u8,
    pub gpe1_base: u8,
    pub cstate_control: u8,
    pub worst_c2_latency: u16,
    pub worst_c3_latency: u16,
    pub flush_size: u16,
    pub flush_stride: u16,
    pub duty_offset: u8,
    pub duty_width: u8,
    pub day_alarm: u8,
    pub month_alarm: u8,
    pub century: u8,
    pub boot_arch_flags: u16,
    pub reserved2: u8,
    pub flags: u32,
    // ACPI 2.0+ fields follow (GenericAddressStructure fields)
}

// Generic Address Structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct GenericAddressStructure {
    pub address_space: u8,
    pub bit_width: u8,
    pub bit_offset: u8,
    pub access_size: u8,
    pub address: u64,
}

// MADT (Multiple APIC Description Table)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Madt {
    pub header: super::SdtHeader,
    pub local_apic_addr: u32,
    pub flags: u32,
    // Variable length interrupt controller structures follow
}

// MADT Entry Types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MadtEntryType {
    LocalApic = 0,
    IoApic = 1,
    InterruptSourceOverride = 2,
    NmiSource = 3,
    LocalApicNmi = 4,
    LocalApicAddressOverride = 5,
    IoSapic = 6,
    LocalSapic = 7,
    PlatformInterruptSources = 8,
    ProcessorLocalX2Apic = 9,
    LocalX2ApicNmi = 10,
}

// MADT Entry Header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtEntryHeader {
    pub entry_type: u8,
    pub length: u8,
}

// Local APIC Entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtLocalApic {
    pub header: MadtEntryHeader,
    pub processor_id: u8,
    pub apic_id: u8,
    pub flags: u32,
}

// I/O APIC Entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtIoApic {
    pub header: MadtEntryHeader,
    pub io_apic_id: u8,
    pub reserved: u8,
    pub io_apic_address: u32,
    pub global_system_interrupt_base: u32,
}

// Interrupt Source Override
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtInterruptSourceOverride {
    pub header: MadtEntryHeader,
    pub bus: u8,
    pub source: u8,
    pub global_system_interrupt: u32,
    pub flags: u16,
}

// MCFG (PCI Express Memory Configuration)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Mcfg {
    pub header: super::SdtHeader,
    pub reserved: [u8; 8],
    // Variable number of configuration base address allocation structures follow
}

// MCFG Base Address Allocation Structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct McfgBaseAddress {
    pub base_address: u64,
    pub segment_group_number: u16,
    pub start_bus_number: u8,
    pub end_bus_number: u8,
    pub reserved: u32,
}

// HPET (High Precision Event Timer)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Hpet {
    pub header: super::SdtHeader,
    pub event_timer_block_id: u32,
    pub base_address: GenericAddressStructure,
    pub hpet_number: u8,
    pub minimum_tick: u16,
    pub page_protection: u8,
}

// FADT Flags
pub const FADT_WBINVD: u32 = 1 << 0;
pub const FADT_WBINVD_FLUSH: u32 = 1 << 1;
pub const FADT_C1_SUPPORTED: u32 = 1 << 2;
pub const FADT_C2_MP_SUPPORTED: u32 = 1 << 3;
pub const FADT_POWER_BUTTON: u32 = 1 << 4;
pub const FADT_SLEEP_BUTTON: u32 = 1 << 5;
pub const FADT_RTC_S4: u32 = 1 << 6;
pub const FADT_TMR_VAL_EXT: u32 = 1 << 7;
pub const FADT_DCK_CAP: u32 = 1 << 8;
pub const FADT_RESET_REG_SUP: u32 = 1 << 9;
pub const FADT_SEALED_CASE: u32 = 1 << 10;
pub const FADT_HEADLESS: u32 = 1 << 11;
pub const FADT_CPU_SW_SLP: u32 = 1 << 12;