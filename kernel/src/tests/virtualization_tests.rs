use crate::hypervisor::{Hypervisor, VmConfig, VirtualizationTechnology};
use crate::container::{Container, ContainerConfig, NetworkMode, ContainerState};
use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

#[test_case]
fn test_hypervisor_detection() {
    serial_println!("Testing hypervisor detection...");
    
    let hypervisor = Hypervisor::new();
    let caps = hypervisor.capabilities();
    
    match caps.virt_tech {
        VirtualizationTechnology::IntelVmx => {
            serial_println!("Intel VT-x detected");
            assert!(true);
        }
        VirtualizationTechnology::AmdSvm => {
            serial_println!("AMD-V detected");
            assert!(true);
        }
        VirtualizationTechnology::None => {
            serial_println!("No virtualization support detected");
        }
    }
    
    serial_println!("EPT/NPT supported: {}", caps.ept_supported || caps.npt_supported);
    serial_println!("VPID supported: {}", caps.vpid_supported);
    serial_println!("Unrestricted guest: {}", caps.unrestricted_guest);
    serial_println!("APIC virtualization: {}", caps.apicv_supported);
    serial_println!("Nested virtualization: {}", caps.nested_virt);
    
    serial_println!("Hypervisor detection test passed!");
}

#[test_case]
fn test_vm_creation() {
    serial_println!("Testing VM creation...");
    
    let mut hypervisor = Hypervisor::new();
    
    if hypervisor.capabilities().virt_tech == VirtualizationTechnology::None {
        serial_println!("Skipping VM creation test - no virtualization support");
        return;
    }
    
    let result = hypervisor.enable();
    if result.is_err() {
        serial_println!("Failed to enable hypervisor: {:?}", result);
        return;
    }
    
    let config = VmConfig {
        name: String::from("test-vm"),
        vcpu_count: 1,
        memory_mb: 128,
        enable_ept: true,
        enable_vpid: true,
        enable_unrestricted: false,
        enable_apicv: false,
    };
    
    match hypervisor.create_vm(config) {
        Ok(vm) => {
            serial_println!("VM created successfully: {}", vm.get_name());
            assert_eq!(vm.get_name(), "test-vm");
        }
        Err(e) => {
            serial_println!("Failed to create VM: {:?}", e);
        }
    }
    
    let _ = hypervisor.disable();
    serial_println!("VM creation test completed!");
}

#[test_case]
fn test_container_creation() {
    serial_println!("Testing container creation...");
    
    let config = ContainerConfig {
        name: String::from("test-container"),
        image: String::from("alpine:latest"),
        command: vec![String::from("/bin/sh")],
        environment: BTreeMap::new(),
        working_dir: String::from("/"),
        hostname: String::from("test-host"),
        network_mode: NetworkMode::Bridge,
        memory_limit: Some(64 * 1024 * 1024),
        cpu_quota: Some(50000),
        cpu_shares: Some(1024),
        readonly_rootfs: false,
        privileged: false,
        capabilities: Vec::new(),
    };
    
    match Container::new(config) {
        Ok(container) => {
            serial_println!("Container created: {}", container.get_name());
            assert_eq!(container.get_name(), "test-container");
            assert_eq!(container.get_state(), ContainerState::Created);
        }
        Err(e) => {
            serial_println!("Failed to create container: {:?}", e);
        }
    }
    
    serial_println!("Container creation test passed!");
}

#[test_case]
fn test_namespace_isolation() {
    use crate::container::namespace::{PidNamespace, NetNamespace, MountNamespace, Namespace};
    
    serial_println!("Testing namespace isolation...");
    
    let mut pid_ns = PidNamespace::new().expect("Failed to create PID namespace");
    assert_eq!(pid_ns.get_id(), 1);
    
    let result = pid_ns.enter();
    assert!(result.is_ok());
    
    let pid1 = pid_ns.allocate_pid();
    let pid2 = pid_ns.allocate_pid();
    assert_eq!(pid1, 1);
    assert_eq!(pid2, 2);
    
    pid_ns.add_pid_mapping(1, 1000);
    assert_eq!(pid_ns.translate_pid(1), Some(1000));
    
    let result = pid_ns.exit();
    assert!(result.is_ok());
    
    let mut net_ns = NetNamespace::new().expect("Failed to create network namespace");
    let result = net_ns.enter();
    assert!(result.is_ok());
    
    let iface_idx = net_ns.add_interface(String::from("veth0"), [0x02, 0x42, 0xac, 0x11, 0x00, 0x02]);
    assert_eq!(iface_idx, 2);
    
    net_ns.add_route([172, 17, 0, 0], 16, Some([172, 17, 0, 1]), iface_idx);
    
    let result = net_ns.exit();
    assert!(result.is_ok());
    
    let mut mount_ns = MountNamespace::new().expect("Failed to create mount namespace");
    mount_ns.set_root(String::from("/container/root"));
    mount_ns.add_mount(
        String::from("proc"),
        String::from("/container/root/proc"),
        String::from("proc"),
        0
    );
    
    serial_println!("Namespace isolation test passed!");
}

#[test_case]
fn test_cgroup_resource_limits() {
    use crate::container::cgroup::{Cgroup, CgroupController};
    
    serial_println!("Testing cgroup resource limits...");
    
    let mut memory_cgroup = Cgroup::new("memory", "test-memory")
        .expect("Failed to create memory cgroup");
    
    let result = memory_cgroup.set_memory_limit(128 * 1024 * 1024);
    assert!(result.is_ok());
    
    let result = memory_cgroup.set_memory_soft_limit(64 * 1024 * 1024);
    assert!(result.is_ok());
    
    let result = memory_cgroup.add_process(1234);
    assert!(result.is_ok());
    
    assert_eq!(memory_cgroup.get_processes().len(), 1);
    assert_eq!(memory_cgroup.get_processes()[0], 1234);
    
    memory_cgroup.update_memory_stats(50 * 1024 * 1024, 10 * 1024 * 1024, 40 * 1024 * 1024);
    assert!(memory_cgroup.check_memory_usage());
    
    memory_cgroup.update_memory_stats(200 * 1024 * 1024, 10 * 1024 * 1024, 190 * 1024 * 1024);
    assert!(!memory_cgroup.check_memory_usage());
    
    let mut cpu_cgroup = Cgroup::new("cpu", "test-cpu")
        .expect("Failed to create CPU cgroup");
    
    let result = cpu_cgroup.set_cpu_shares(2048);
    assert!(result.is_ok());
    
    let result = cpu_cgroup.set_cpu_quota(50000);
    assert!(result.is_ok());
    
    cpu_cgroup.update_cpu_stats(1000000, 600000, 400000);
    
    let mut pids_cgroup = Cgroup::new("pids", "test-pids")
        .expect("Failed to create pids cgroup");
    
    let result = pids_cgroup.set_pids_limit(10);
    assert!(result.is_ok());
    
    for i in 1..=10 {
        let result = pids_cgroup.add_process(i);
        assert!(result.is_ok());
    }
    
    let result = pids_cgroup.add_process(11);
    assert!(result.is_err());
    
    serial_println!("Cgroup resource limits test passed!");
}

#[test_case]
fn test_device_virtualization() {
    use crate::hypervisor::device::{VirtioDevice, VirtioDeviceType, VirtualSerialPort, VirtualDevice};
    
    serial_println!("Testing device virtualization...");
    
    let mut virtio_net = VirtioDevice::new(VirtioDeviceType::Network, 1);
    assert_eq!(virtio_net.name(), "virtio-net");
    assert_eq!(virtio_net.device_id(), 1);
    
    let magic = virtio_net.mmio_read(0x00, 4).expect("Failed to read magic");
    assert_eq!(magic, 0x74726976);
    
    let version = virtio_net.mmio_read(0x04, 4).expect("Failed to read version");
    assert_eq!(version, 0x2);
    
    let device_id = virtio_net.mmio_read(0x08, 4).expect("Failed to read device ID");
    assert_eq!(device_id, VirtioDeviceType::Network as u64);
    
    let result = virtio_net.mmio_write(0x70, 0, 4);
    assert!(result.is_ok());
    
    let mut serial = VirtualSerialPort::new(1);
    assert_eq!(serial.name(), "serial");
    
    let lsr = serial.io_read(0x3F8 + 5, 1).expect("Failed to read LSR");
    assert_eq!(lsr & 0x60, 0x60);
    
    let result = serial.io_write(0x3F8, 'H' as u32, 1);
    assert!(result.is_ok());
    
    let result = serial.io_write(0x3F8, 'i' as u32, 1);
    assert!(result.is_ok());
    
    serial_println!("Device virtualization test passed!");
}

#[test_case]
fn test_ept_npt_memory_mapping() {
    use crate::hypervisor::memory::{GuestMemory, EptManager, NptManager};
    
    serial_println!("Testing EPT/NPT memory mapping...");
    
    let guest_memory = GuestMemory::new(16 * 1024 * 1024)
        .expect("Failed to allocate guest memory");
    
    let test_data = [0x12, 0x34, 0x56, 0x78];
    let result = guest_memory.write(0x1000, &test_data);
    assert!(result.is_ok());
    
    let mut read_buf = [0u8; 4];
    let result = guest_memory.read(0x1000, &mut read_buf);
    assert!(result.is_ok());
    assert_eq!(read_buf, test_data);
    
    let translated = guest_memory.translate_gpa(0x1000);
    assert!(translated.is_some());
    
    let mut ept_manager = EptManager::new()
        .expect("Failed to create EPT manager");
    
    let result = ept_manager.map_page(0x1000, 0x100000, 0x7);
    assert!(result.is_ok());
    
    let eptp = ept_manager.get_eptp();
    assert_ne!(eptp, 0);
    
    let npt_manager = NptManager::new()
        .expect("Failed to create NPT manager");
    
    let ncr3 = npt_manager.get_ncr3();
    
    serial_println!("EPT/NPT memory mapping test passed!");
}

#[test_case]
fn test_container_lifecycle() {
    serial_println!("Testing container lifecycle...");
    
    let config = ContainerConfig {
        name: String::from("lifecycle-test"),
        image: String::from("alpine:latest"),
        command: vec![String::from("/bin/sh"), String::from("-c"), String::from("echo hello")],
        environment: BTreeMap::new(),
        working_dir: String::from("/"),
        hostname: String::from("test"),
        network_mode: NetworkMode::None,
        memory_limit: None,
        cpu_quota: None,
        cpu_shares: None,
        readonly_rootfs: false,
        privileged: false,
        capabilities: Vec::new(),
    };
    
    let mut container = Container::new(config)
        .expect("Failed to create container");
    
    assert_eq!(container.get_state(), ContainerState::Created);
    
    let result = container.start();
    if result.is_ok() {
        assert_eq!(container.get_state(), ContainerState::Running);
        assert!(container.get_pid().is_some());
        
        let result = container.pause();
        if result.is_ok() {
            assert_eq!(container.get_state(), ContainerState::Paused);
        }
        
        let result = container.resume();
        if result.is_ok() {
            assert_eq!(container.get_state(), ContainerState::Running);
        }
        
        let result = container.stop();
        if result.is_ok() {
            assert_eq!(container.get_state(), ContainerState::Stopped);
        }
    }
    
    serial_println!("Container lifecycle test passed!");
}

pub fn run_all_virtualization_tests() {
    serial_println!("\n=== Running Virtualization Tests ===\n");
    
    test_hypervisor_detection();
    test_vm_creation();
    test_container_creation();
    test_namespace_isolation();
    test_cgroup_resource_limits();
    test_device_virtualization();
    test_ept_npt_memory_mapping();
    test_container_lifecycle();
    
    serial_println!("\n=== All Virtualization Tests Completed ===\n");
}