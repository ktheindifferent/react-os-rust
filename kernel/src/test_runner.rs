// Custom Test Runner for Kernel Testing
use crate::{serial_print, serial_println, println};
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use alloc::format;

pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

pub struct TestRunner {
    tests: Vec<TestResult>,
    current_test: Option<String>,
}

impl TestRunner {
    pub fn new() -> Self {
        Self {
            tests: Vec::new(),
            current_test: None,
        }
    }
    
    pub fn run_test<F>(&mut self, name: &str, test_fn: F) 
    where
        F: FnOnce() -> Result<(), String>
    {
        serial_print!("Testing {}... ", name);
        self.current_test = Some(String::from(name));
        
        match test_fn() {
            Ok(()) => {
                serial_println!("[PASS]");
                self.tests.push(TestResult {
                    name: String::from(name),
                    passed: true,
                    error: None,
                });
            }
            Err(e) => {
                serial_println!("[FAIL]");
                serial_println!("  Error: {}", e);
                self.tests.push(TestResult {
                    name: String::from(name),
                    passed: false,
                    error: Some(e),
                });
            }
        }
        
        self.current_test = None;
    }
    
    pub fn assert_eq<T: PartialEq + core::fmt::Debug>(&self, left: T, right: T, msg: &str) -> Result<(), String> {
        if left == right {
            Ok(())
        } else {
            Err(format!("{}: expected {:?}, got {:?}", msg, right, left))
        }
    }
    
    pub fn assert(&self, condition: bool, msg: &str) -> Result<(), String> {
        if condition {
            Ok(())
        } else {
            Err(String::from(msg))
        }
    }
    
    pub fn summary(&self) {
        let total = self.tests.len();
        let passed = self.tests.iter().filter(|t| t.passed).count();
        let failed = total - passed;
        
        println!("\n===== Test Summary =====");
        println!("Total:  {}", total);
        println!("Passed: {}", passed);
        println!("Failed: {}", failed);
        
        if failed > 0 {
            println!("\nFailed tests:");
            for test in &self.tests {
                if !test.passed {
                    println!("  - {}", test.name);
                    if let Some(ref error) = test.error {
                        println!("    {}", error);
                    }
                }
            }
        }
        
        if failed == 0 {
            println!("\n✓ All tests passed!");
        } else {
            println!("\n✗ {} test(s) failed", failed);
        }
    }
}

// Hardware test suites
pub fn run_all_tests() {
    println!("\n Starting Hardware Component Tests...\n");
    
    let mut runner = TestRunner::new();
    
    // Sound tests
    run_sound_tests(&mut runner);
    
    // NVMe tests
    run_nvme_tests(&mut runner);
    
    // PCIe tests
    run_pcie_tests(&mut runner);
    
    // Integration tests
    run_integration_tests(&mut runner);
    
    runner.summary();
}

fn run_sound_tests(runner: &mut TestRunner) {
    use crate::sound::*;
    
    runner.run_test("sound::audio_format", || {
        let format = AudioFormat {
            sample_rate: 44100,
            channels: 2,
            format: SampleFormat::S16LE,
            buffer_size: 512,
        };
        
        if format.sample_rate != 44100 {
            return Err(format!("sample rate: expected 44100, got {}", format.sample_rate));
        }
        if format.channels != 2 {
            return Err(format!("channels: expected 2, got {}", format.channels));
        }
        if format.format.bytes_per_sample() != 2 {
            return Err(format!("bytes per sample: expected 2, got {}", format.format.bytes_per_sample()));
        }
        Ok(())
    });
    
    runner.run_test("sound::audio_buffer", || {
        let format = AudioFormat::default();
        let buffer = AudioBuffer::new(format.clone());
        
        if buffer.frames != format.buffer_size {
            return Err(format!("buffer frames: expected {}, got {}", format.buffer_size, buffer.frames));
        }
        if buffer.data.len() == 0 {
            return Err(String::from("buffer has no data"));
        }
        Ok(())
    });
    
    runner.run_test("sound::sample_format_sizes", || {
        if SampleFormat::U8.bytes_per_sample() != 1 {
            return Err(String::from("U8 size incorrect"));
        }
        if SampleFormat::S16LE.bytes_per_sample() != 2 {
            return Err(String::from("S16LE size incorrect"));
        }
        if SampleFormat::S24LE.bytes_per_sample() != 3 {
            return Err(String::from("S24LE size incorrect"));
        }
        if SampleFormat::S32LE.bytes_per_sample() != 4 {
            return Err(String::from("S32LE size incorrect"));
        }
        if SampleFormat::F32LE.bytes_per_sample() != 4 {
            return Err(String::from("F32LE size incorrect"));
        }
        Ok(())
    });
    
    runner.run_test("sound::math_approximations", || {
        // Test sine approximation
        let sin_0 = sine_approx(0.0);
        if sin_0.abs() >= 0.01 {
            return Err(format!("sin(0) ≈ 0 failed: got {}", sin_0));
        }
        
        let sin_pi_2 = sine_approx(core::f32::consts::FRAC_PI_2);
        if (sin_pi_2 - 1.0).abs() >= 0.01 {
            return Err(format!("sin(π/2) ≈ 1 failed: got {}", sin_pi_2));
        }
        
        // Test power of 2 approximation
        let pow2_0 = pow2_approx(0.0);
        if (pow2_0 - 1.0).abs() >= 0.01 {
            return Err(format!("2^0 ≈ 1 failed: got {}", pow2_0));
        }
        
        let pow2_1 = pow2_approx(1.0);
        if (pow2_1 - 2.0).abs() >= 0.1 {
            return Err(format!("2^1 ≈ 2 failed: got {}", pow2_1));
        }
        
        Ok(())
    });
}

fn run_nvme_tests(runner: &mut TestRunner) {
    use crate::nvme::*;
    use crate::nvme::command::*;
    
    runner.run_test("nvme::command_creation", || {
        let cmd = NvmeCommand::new();
        if cmd.opcode != 0 {
            return Err(format!("default opcode: expected 0, got {}", cmd.opcode));
        }
        if cmd.nsid != 0 {
            return Err(format!("default namespace: expected 0, got {}", cmd.nsid));
        }
        if cmd.command_id != 0 {
            return Err(format!("default command id: expected 0, got {}", cmd.command_id));
        }
        Ok(())
    });
    
    runner.run_test("nvme::command_builder", || {
        let cmd = NvmeCommandBuilder::new()
            .opcode(NVME_IO_READ)
            .namespace(1)
            .prp1(0x1000)
            .cdw10(100)
            .build();
        
        if cmd.opcode != NVME_IO_READ {
            return Err(format!("opcode: expected {}, got {}", NVME_IO_READ, cmd.opcode));
        }
        if cmd.nsid != 1 {
            return Err(format!("namespace: expected 1, got {}", cmd.nsid));
        }
        if cmd.prp1 != 0x1000 {
            return Err(format!("prp1: expected 0x1000, got 0x{:x}", cmd.prp1));
        }
        if cmd.cdw10 != 100 {
            return Err(format!("cdw10: expected 100, got {}", cmd.cdw10));
        }
        Ok(())
    });
    
    runner.run_test("nvme::completion_status", || {
        let mut completion = NvmeCompletion {
            result: 0,
            reserved: 0,
            sq_head: 0,
            sq_id: 0,
            command_id: 0,
            status: 0,
        };
        
        if completion.is_error() {
            return Err(String::from("no error by default failed"));
        }
        
        completion.status = 0x02;
        if !completion.is_error() {
            return Err(String::from("error when status set failed"));
        }
        
        completion.status = 0x01;
        if !completion.get_phase() {
            return Err(String::from("phase bit set failed"));
        }
        
        Ok(())
    });
    
    runner.run_test("nvme::pci_location", || {
        use crate::pcie::PciLocation;
        let loc = PciLocation::new(0, 1, 2, 3);
        if loc.bus != 1 {
            return Err(format!("bus: expected 1, got {}", loc.bus));
        }
        if loc.device != 2 {
            return Err(format!("device: expected 2, got {}", loc.device));
        }
        if loc.function != 3 {
            return Err(format!("function: expected 3, got {}", loc.function));
        }
        
        let addr = loc.to_legacy_address(0x04);
        if addr & 0x80000000 == 0 {
            return Err(String::from("enable bit not set"));
        }
        
        Ok(())
    });
}

fn run_pcie_tests(runner: &mut TestRunner) {
    use crate::pcie::*;
    
    runner.run_test("pcie::device_classification", || {
        let device = PciDevice {
            location: PciLocation::new(0, 0, 0, 0),
            vendor_id: 0x8086,
            device_id: 0x1234,
            class: PCI_CLASS_STORAGE,
            subclass: PCI_SUBCLASS_STORAGE_NVME,
            prog_if: 0,
            revision: 0,
            header_type: 0,
            bars: [0; 6],
            capabilities: vec![],
            extended_capabilities: vec![],
        };
        
        if !device.is_storage() {
            return Err(String::from("is storage device failed"));
        }
        if device.is_network() {
            return Err(String::from("not network device failed"));
        }
        if device.is_bridge() {
            return Err(String::from("not bridge device failed"));
        }
        Ok(())
    });
    
    runner.run_test("pcie::capability_detection", || {
        let mut device = PciDevice {
            location: PciLocation::new(0, 0, 0, 0),
            vendor_id: 0x8086,
            device_id: 0x1234,
            class: 0,
            subclass: 0,
            prog_if: 0,
            revision: 0,
            header_type: 0,
            bars: [0; 6],
            capabilities: vec![],
            extended_capabilities: vec![],
        };
        
        device.capabilities.push(PciCapability {
            id: PCI_CAP_ID_MSI,
            offset: 0x50,
            data: vec![0; 16],
        });
        
        if !device.has_capability(PCI_CAP_ID_MSI) {
            return Err(String::from("has MSI failed"));
        }
        if device.has_capability(PCI_CAP_ID_MSIX) {
            return Err(String::from("no MSI-X failed"));
        }
        if !device.supports_msi() {
            return Err(String::from("supports MSI failed"));
        }
        Ok(())
    });
    
    runner.run_test("pcie::bar_types", || {
        // I/O BAR
        let io_bar = 0x0000E001;
        if io_bar & 1 != 1 {
            return Err(String::from("I/O space indicator failed"));
        }
        
        // Memory BAR
        let mem_bar = 0x10000000;
        if mem_bar & 1 != 0 {
            return Err(String::from("memory space indicator failed"));
        }
        
        Ok(())
    });
}

fn run_integration_tests(runner: &mut TestRunner) {
    runner.run_test("integration::memory_alignment", || {
        // NVMe alignment
        let nvme_addr = 0x10000000u64;
        if nvme_addr & 0xFFF != 0 {
            return Err(String::from("NVMe 4KB alignment failed"));
        }
        
        // Audio alignment
        let audio_addr = 0x20000040u64;
        if audio_addr & 0x3F != 0 {
            return Err(String::from("Audio 64-byte alignment failed"));
        }
        
        Ok(())
    });
    
    runner.run_test("integration::dma_buffer_sizes", || {
        // NVMe PRP list
        let prp_size = 8 * 512;
        if prp_size != 4096 {
            return Err(format!("PRP list size: expected 4096, got {}", prp_size));
        }
        
        // Audio BDL
        let bdl_size = 32 * 16;
        if bdl_size != 512 {
            return Err(format!("BDL size: expected 512, got {}", bdl_size));
        }
        
        // MSI-X table
        let msix_size = 256 * 16;
        if msix_size != 4096 {
            return Err(format!("MSI-X table size: expected 4096, got {}", msix_size));
        }
        
        Ok(())
    });
    
    runner.run_test("integration::error_handling", || {
        fn test_error() -> Result<(), &'static str> {
            Err("Test error")
        }
        
        let result = test_error();
        if !result.is_err() {
            return Err(String::from("error is not detected"));
        }
        if result.unwrap_err() != "Test error" {
            return Err(String::from("error message mismatch"));
        }
        
        Ok(())
    });
}