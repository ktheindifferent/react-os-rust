use super::{NtStatus, object::{ObjectManager, OBJECT_MANAGER}, process::{ProcessManager, PROCESS_MANAGER}};
use crate::memory::{MemoryManager, MEMORY_MANAGER};
use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicU64, Ordering};

// NT Executive - The core NT kernel executive layer
pub struct NtExecutive {
    // System information
    system_process_id: super::process::ProcessId,
    boot_time: AtomicU64,
    system_call_count: AtomicU64,
    
    // Executive subsystems status
    object_manager_initialized: bool,
    process_manager_initialized: bool,
    memory_manager_initialized: bool,
    io_manager_initialized: bool,
    security_manager_initialized: bool,
    
    // System configuration
    number_of_processors: u32,
    page_size: u64,
    allocation_granularity: u64,
    minimum_application_address: u64,
    maximum_application_address: u64,
    active_processor_mask: u64,
    processor_type: ProcessorType,
    processor_level: u16,
    processor_revision: u16,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessorType {
    Intel386 = 386,
    Intel486 = 486,
    IntelPentium = 586,
    IntelPentiumPro = 686,
    IntelIA64 = 2200,
    AMD64 = 8664,
}

impl NtExecutive {
    pub fn new() -> Self {
        Self {
            system_process_id: super::process::ProcessId::SYSTEM,
            boot_time: AtomicU64::new(0),
            system_call_count: AtomicU64::new(0),
            object_manager_initialized: false,
            process_manager_initialized: false,
            memory_manager_initialized: false,
            io_manager_initialized: false,
            security_manager_initialized: false,
            number_of_processors: 1,
            page_size: 4096,
            allocation_granularity: 65536,
            minimum_application_address: 0x10000,
            maximum_application_address: 0x7FFEFFFF,
            active_processor_mask: 1,
            processor_type: ProcessorType::AMD64,
            processor_level: 6,
            processor_revision: 0,
        }
    }
    
    pub fn initialize(&mut self) -> NtStatus {
        use crate::serial_println;
        
        serial_println!("NT Executive: Starting initialization");
        
        // Initialize Object Manager
        {
            let _om = OBJECT_MANAGER.lock();
            self.object_manager_initialized = true;
            serial_println!("NT Executive: Object Manager initialized");
        }
        
        // Initialize Process Manager
        {
            let _pm = PROCESS_MANAGER.lock();
            self.process_manager_initialized = true;
            serial_println!("NT Executive: Process Manager initialized");
        }
        
        // Initialize Memory Manager
        {
            let _mm = MEMORY_MANAGER.lock();
            self.memory_manager_initialized = true;
            serial_println!("NT Executive: Memory Manager initialized");
        }
        
        // Set boot time (simplified)
        self.boot_time.store(0, Ordering::SeqCst);
        
        serial_println!("NT Executive: Initialization complete");
        NtStatus::Success
    }
    
    pub fn is_fully_initialized(&self) -> bool {
        self.object_manager_initialized && 
        self.process_manager_initialized && 
        self.memory_manager_initialized
    }
    
    pub fn increment_system_call_count(&self) {
        self.system_call_count.fetch_add(1, Ordering::SeqCst);
    }
    
    pub fn get_system_call_count(&self) -> u64 {
        self.system_call_count.load(Ordering::SeqCst)
    }
    
    pub fn get_system_info(&self) -> SystemBasicInformation {
        SystemBasicInformation {
            reserved: 0,
            timer_resolution: 156250, // 15.625 ms in 100ns units
            page_size: self.page_size as u32,
            number_of_physical_pages: 32768, // 128MB / 4KB
            lowest_physical_page_number: 0,
            highest_physical_page_number: 32767,
            allocation_granularity: self.allocation_granularity as u32,
            minimum_user_mode_address: self.minimum_application_address,
            maximum_user_mode_address: self.maximum_application_address,
            active_processors_affinity_mask: self.active_processor_mask,
            number_of_processors: self.number_of_processors as u8,
        }
    }
    
    pub fn get_processor_info(&self) -> SystemProcessorInformation {
        SystemProcessorInformation {
            processor_architecture: ProcessorArchitecture::AMD64,
            processor_level: self.processor_level,
            processor_revision: self.processor_revision,
            maximum_processors: self.number_of_processors as u16,
            processor_feature_bits: 0x001FFFFF, // Standard x64 features
        }
    }
    
    pub fn get_performance_info(&self) -> SystemPerformanceInformation {
        SystemPerformanceInformation {
            idle_process_time: 0,
            io_read_transfer_count: 0,
            io_write_transfer_count: 0,
            io_other_transfer_count: 0,
            io_read_operation_count: 0,
            io_write_operation_count: 0,
            io_other_operation_count: 0,
            available_pages: 16384, // 64MB available
            committed_pages: 16384,  // 64MB committed
            commit_limit: 65536,     // 256MB limit
            peak_commitment: 16384,
            page_fault_count: 1000,
            copy_on_write_count: 100,
            transition_count: 50,
            cache_transition_count: 25,
            demand_zero_count: 500,
            page_read_count: 200,
            page_read_io_count: 150,
            cache_read_count: 300,
            cache_read_io_count: 250,
            page_file_page_count: 0,
            page_file_page_count_peak: 0,
            kernel_stack: 128,
            kernel_paged: 8192,
            kernel_non_paged: 2048,
            system_code_page: 4096,
            total_system_driver_pages: 1024,
            total_system_code_pages: 4096,
            small_non_paged_lookaside_list_allocate_hits: 0,
            small_paged_lookaside_list_allocate_hits: 0,
            reserved3: 0,
            mm_system_code_page: 4096,
            mm_system_cache_page: 8192,
            mm_paged_pool_page: 8192,
            mm_system_driver_page: 1024,
            cc_fast_read_no_wait: 0,
            cc_fast_read_wait: 0,
            cc_fast_read_resource_miss: 0,
            cc_fast_read_not_possible: 0,
            cc_fast_mdl_read_no_wait: 0,
            cc_fast_mdl_read_wait: 0,
            cc_fast_mdl_read_resource_miss: 0,
            cc_fast_mdl_read_not_possible: 0,
            cc_map_data_no_wait: 0,
            cc_map_data_wait: 0,
            cc_map_data_no_wait_miss: 0,
            cc_map_data_wait_miss: 0,
            cc_pin_mapped_data_count: 0,
            cc_pin_read_no_wait: 0,
            cc_pin_read_wait: 0,
            cc_pin_read_no_wait_miss: 0,
            cc_pin_read_wait_miss: 0,
            cc_copy_read_no_wait: 0,
            cc_copy_read_wait: 0,
            cc_copy_read_no_wait_miss: 0,
            cc_copy_read_wait_miss: 0,
            cc_mdl_read_no_wait: 0,
            cc_mdl_read_wait: 0,
            cc_mdl_read_no_wait_miss: 0,
            cc_mdl_read_wait_miss: 0,
            cc_read_ahead_ios: 0,
            cc_lazy_write_ios: 0,
            cc_lazy_write_pages: 0,
            cc_data_flushes: 0,
            cc_data_pages: 0,
            context_switches: 10000,
            first_level_tb_fills: 0,
            second_level_tb_fills: 0,
            system_calls: self.get_system_call_count() as u32,
        }
    }
}

// System information structures - Windows compatible
#[repr(C)]
#[derive(Debug, Clone)]
pub struct SystemBasicInformation {
    pub reserved: u32,
    pub timer_resolution: u32,
    pub page_size: u32,
    pub number_of_physical_pages: u32,
    pub lowest_physical_page_number: u32,
    pub highest_physical_page_number: u32,
    pub allocation_granularity: u32,
    pub minimum_user_mode_address: u64,
    pub maximum_user_mode_address: u64,
    pub active_processors_affinity_mask: u64,
    pub number_of_processors: u8,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct SystemProcessorInformation {
    pub processor_architecture: ProcessorArchitecture,
    pub processor_level: u16,
    pub processor_revision: u16,
    pub maximum_processors: u16,
    pub processor_feature_bits: u32,
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessorArchitecture {
    Intel = 0,
    MIPS = 1,
    Alpha = 2,
    PPC = 3,
    SHX = 4,
    ARM = 5,
    IA64 = 6,
    Alpha64 = 7,
    MSIL = 8,
    AMD64 = 9,
    IA32OnWin64 = 10,
    Neutral = 11,
    ARM64 = 12,
    ARM32OnWin64 = 13,
    IA32OnARM64 = 14,
    Unknown = 0xFFFF,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct SystemPerformanceInformation {
    pub idle_process_time: u64,
    pub io_read_transfer_count: u64,
    pub io_write_transfer_count: u64,
    pub io_other_transfer_count: u64,
    pub io_read_operation_count: u32,
    pub io_write_operation_count: u32,
    pub io_other_operation_count: u32,
    pub available_pages: u32,
    pub committed_pages: u32,
    pub commit_limit: u32,
    pub peak_commitment: u32,
    pub page_fault_count: u32,
    pub copy_on_write_count: u32,
    pub transition_count: u32,
    pub cache_transition_count: u32,
    pub demand_zero_count: u32,
    pub page_read_count: u32,
    pub page_read_io_count: u32,
    pub cache_read_count: u32,
    pub cache_read_io_count: u32,
    pub page_file_page_count: u32,
    pub page_file_page_count_peak: u32,
    pub kernel_stack: u32,
    pub kernel_paged: u32,
    pub kernel_non_paged: u32,
    pub system_code_page: u32,
    pub total_system_driver_pages: u32,
    pub total_system_code_pages: u32,
    pub small_non_paged_lookaside_list_allocate_hits: u32,
    pub small_paged_lookaside_list_allocate_hits: u32,
    pub reserved3: u32,
    pub mm_system_code_page: u32,
    pub mm_system_cache_page: u32,
    pub mm_paged_pool_page: u32,
    pub mm_system_driver_page: u32,
    pub cc_fast_read_no_wait: u32,
    pub cc_fast_read_wait: u32,
    pub cc_fast_read_resource_miss: u32,
    pub cc_fast_read_not_possible: u32,
    pub cc_fast_mdl_read_no_wait: u32,
    pub cc_fast_mdl_read_wait: u32,
    pub cc_fast_mdl_read_resource_miss: u32,
    pub cc_fast_mdl_read_not_possible: u32,
    pub cc_map_data_no_wait: u32,
    pub cc_map_data_wait: u32,
    pub cc_map_data_no_wait_miss: u32,
    pub cc_map_data_wait_miss: u32,
    pub cc_pin_mapped_data_count: u32,
    pub cc_pin_read_no_wait: u32,
    pub cc_pin_read_wait: u32,
    pub cc_pin_read_no_wait_miss: u32,
    pub cc_pin_read_wait_miss: u32,
    pub cc_copy_read_no_wait: u32,
    pub cc_copy_read_wait: u32,
    pub cc_copy_read_no_wait_miss: u32,
    pub cc_copy_read_wait_miss: u32,
    pub cc_mdl_read_no_wait: u32,
    pub cc_mdl_read_wait: u32,
    pub cc_mdl_read_no_wait_miss: u32,
    pub cc_mdl_read_wait_miss: u32,
    pub cc_read_ahead_ios: u32,
    pub cc_lazy_write_ios: u32,
    pub cc_lazy_write_pages: u32,
    pub cc_data_flushes: u32,
    pub cc_data_pages: u32,
    pub context_switches: u32,
    pub first_level_tb_fills: u32,
    pub second_level_tb_fills: u32,
    pub system_calls: u32,
}

lazy_static! {
    pub static ref NT_EXECUTIVE: Mutex<NtExecutive> = Mutex::new(NtExecutive::new());
}

// NT Executive API functions
pub fn ex_initialize_system() -> NtStatus {
    let mut executive = NT_EXECUTIVE.lock();
    executive.initialize()
}

pub fn ex_is_system_initialized() -> bool {
    let executive = NT_EXECUTIVE.lock();
    executive.is_fully_initialized()
}

pub fn ex_get_system_information(info_class: SystemInformationClass) -> Result<Vec<u8>, NtStatus> {
    let executive = NT_EXECUTIVE.lock();
    
    match info_class {
        SystemInformationClass::SystemBasicInformation => {
            let info = executive.get_system_info();
            let bytes = unsafe {
                core::slice::from_raw_parts(
                    &info as *const _ as *const u8,
                    core::mem::size_of::<SystemBasicInformation>()
                )
            };
            Ok(bytes.to_vec())
        }
        SystemInformationClass::SystemProcessorInformation => {
            let info = executive.get_processor_info();
            let bytes = unsafe {
                core::slice::from_raw_parts(
                    &info as *const _ as *const u8,
                    core::mem::size_of::<SystemProcessorInformation>()
                )
            };
            Ok(bytes.to_vec())
        }
        SystemInformationClass::SystemPerformanceInformation => {
            let info = executive.get_performance_info();
            let bytes = unsafe {
                core::slice::from_raw_parts(
                    &info as *const _ as *const u8,
                    core::mem::size_of::<SystemPerformanceInformation>()
                )
            };
            Ok(bytes.to_vec())
        }
        _ => Err(NtStatus::InvalidInfoClass),
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemInformationClass {
    SystemBasicInformation = 0,
    SystemProcessorInformation = 1,
    SystemPerformanceInformation = 2,
    SystemTimeOfDayInformation = 3,
    SystemPathInformation = 4,
    SystemProcessInformation = 5,
    SystemCallCountInformation = 6,
    SystemDeviceInformation = 7,
    SystemProcessorPerformanceInformation = 8,
    SystemFlagsInformation = 9,
    SystemCallTimeInformation = 10,
    SystemModuleInformation = 11,
    SystemLocksInformation = 12,
    SystemStackTraceInformation = 13,
    SystemPagedPoolInformation = 14,
    SystemNonPagedPoolInformation = 15,
    SystemHandleInformation = 16,
    SystemObjectInformation = 17,
    SystemPageFileInformation = 18,
    SystemVdmInstemulInformation = 19,
    SystemVdmBopInformation = 20,
    SystemFileCacheInformation = 21,
    SystemPoolTagInformation = 22,
    SystemInterruptInformation = 23,
    SystemDpcBehaviorInformation = 24,
    SystemFullMemoryInformation = 25,
    SystemLoadGdiDriverInformation = 26,
    SystemUnloadGdiDriverInformation = 27,
    SystemTimeAdjustmentInformation = 28,
    SystemSummaryMemoryInformation = 29,
    SystemMirrorMemoryInformation = 30,
    SystemPerformanceTraceInformation = 31,
    SystemObsolete0 = 32,
    SystemExceptionInformation = 33,
    SystemCrashDumpStateInformation = 34,
    SystemKernelDebuggerInformation = 35,
    SystemContextSwitchInformation = 36,
    SystemRegistryQuotaInformation = 37,
    SystemExtendServiceTableInformation = 38,
    SystemPrioritySeperation = 39,
    SystemVerifierAddDriverInformation = 40,
    SystemVerifierRemoveDriverInformation = 41,
    SystemProcessorIdleInformation = 42,
    SystemLegacyDriverInformation = 43,
    SystemCurrentTimeZoneInformation = 44,
    SystemLookasideInformation = 45,
    SystemTimeSlipNotification = 46,
    SystemSessionCreate = 47,
    SystemSessionDetach = 48,
    SystemSessionInformation = 49,
    SystemRangeStartInformation = 50,
}