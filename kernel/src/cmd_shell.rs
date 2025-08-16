use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::{print, println, serial_println};

const MAX_COMMAND_LENGTH: usize = 256;
const COMMAND_HISTORY_SIZE: usize = 10;

pub struct Shell {
    command_buffer: String,
    cursor_visible: bool,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            command_buffer: String::new(),
            cursor_visible: true,
        }
    }

    pub fn run(&mut self) {
        // Deprecated - initialization now done in init()
    }

    fn print_welcome(&self) {
        println!("\n=====================================");
        println!("     ReactOS Rust Shell v0.1.0      ");
        println!("=====================================");
        println!("Type 'help' for available commands\n");
    }

    fn print_prompt(&self) {
        print!("ReactOS> ");
    }

    pub fn handle_key(&mut self, key: char) {
        match key {
            '\n' => {
                println!(); // New line after command
                self.execute_command();
                self.command_buffer.clear();
                self.print_prompt();
            }
            '\x08' => { // Backspace
                if !self.command_buffer.is_empty() {
                    self.command_buffer.pop();
                    // Move cursor back, print space, move back again
                    print!("\x08 \x08");
                }
            }
            '\x7f' => { // Delete (alternative backspace)
                if !self.command_buffer.is_empty() {
                    self.command_buffer.pop();
                    print!("\x08 \x08");
                }
            }
            _ => {
                if self.command_buffer.len() < MAX_COMMAND_LENGTH && key.is_ascii() && !key.is_control() {
                    self.command_buffer.push(key);
                    print!("{}", key);
                }
            }
        }
    }

    fn execute_command(&mut self) {
        let command = self.command_buffer.trim();
        
        if command.is_empty() {
            return;
        }

        // Parse and execute command
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        match parts[0] {
            "help" => self.cmd_help(),
            "clear" | "cls" => self.cmd_clear(),
            "echo" => self.cmd_echo(&parts[1..]),
            "ver" | "version" => self.cmd_version(),
            "mem" | "memory" => self.cmd_memory(),
            "ps" | "processes" => self.cmd_processes(),
            "uptime" => self.cmd_uptime(),
            "ls" | "dir" => self.cmd_ls(&parts[1..]),
            "cat" | "type" => self.cmd_cat(&parts[1..]),
            "shutdown" => self.cmd_shutdown(),
            "reboot" => self.cmd_reboot(),
            "test" => self.cmd_test(),
            "exec" | "run" => self.cmd_execute(&parts[1..]),
            _ => {
                // Try to execute as a binary if it ends with .exe
                if parts[0].ends_with(".exe") || parts[0].ends_with(".EXE") {
                    self.cmd_execute(&parts[0..]);
                } else {
                    println!("Unknown command: '{}'. Type 'help' for available commands.", parts[0]);
                }
            }
        }
    }

    fn cmd_help(&self) {
        println!("Available commands:");
        println!("  help          - Show this help message");
        println!("  clear/cls     - Clear the screen");
        println!("  echo [text]   - Print text to screen");
        println!("  ver/version   - Show system version");
        println!("  mem/memory    - Show memory usage");
        println!("  ps/processes  - List running processes");
        println!("  uptime        - Show system uptime");
        println!("  ls/dir [path] - List directory contents");
        println!("  cat/type file - Display file contents");
        println!("  exec/run file - Execute a Windows .exe file");
        println!("  test          - Run system tests");
        println!("  shutdown      - Shutdown the system");
        println!("  reboot        - Reboot the system");
        println!("\nYou can also run .exe files directly: hello.exe");
    }

    fn cmd_clear(&self) {
        // Clear screen using VGA buffer clear
        crate::vga_buffer::clear_screen();
        // Re-show the shell header
        self.print_welcome();
    }

    fn cmd_echo(&self, args: &[&str]) {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                print!(" ");
            }
            print!("{}", arg);
        }
        println!();
    }

    fn cmd_version(&self) {
        println!("ReactOS Rust Edition v0.4.15");
        println!("Kernel: Rust OS Kernel v0.1.0");
        println!("Build: Debug");
        println!("Architecture: x86_64");
    }

    fn cmd_memory(&self) {
        println!("Memory Information:");
        println!("  Total Heap: 1024 KB");
        println!("  Used: ~256 KB (estimated)");
        println!("  Free: ~768 KB (estimated)");
        println!("  Page Size: 4096 bytes");
    }

    fn cmd_processes(&self) {
        use crate::process::executor::EXECUTOR;
        
        println!("Process List:");
        println!("  PID | Name            | State");
        println!("  ----|-----------------|--------");
        
        let executor = EXECUTOR.lock();
        for (pid, name, state) in executor.list_processes() {
            println!("  {:3} | {:15} | {}", pid, name, state);
        }
    }

    fn cmd_uptime(&self) {
        // This would normally calculate from timer ticks
        println!("System uptime: 00:00:42");
    }

    fn cmd_test(&self) {
        use crate::test_runner::run_all_tests;
        run_all_tests();
    }

    fn cmd_shutdown(&self) {
        println!("Shutting down...");
        serial_println!("System shutdown requested");
        // In real implementation, would properly shutdown
        loop {
            x86_64::instructions::hlt();
        }
    }

    fn cmd_reboot(&self) {
        println!("Rebooting...");
        serial_println!("System reboot requested");
        // Trigger a triple fault to reboot (simple method)
        unsafe {
            core::ptr::null_mut::<u8>().write(0);
        }
    }
    
    fn cmd_ls(&self, args: &[&str]) {
        use crate::fs::vfs::VFS;
        
        let path = if args.is_empty() { "/" } else { args[0] };
        
        let vfs = VFS.lock();
        match vfs.list_directory(path) {
            Ok(files) => {
                println!("Directory listing of {}:", path);
                println!("  Type  Size     Name");
                println!("  ----  -------- ----");
                
                for file in files {
                    let type_str = match file.file_type {
                        crate::fs::FileType::Directory => "DIR ",
                        crate::fs::FileType::Regular => "FILE",
                        _ => "????",
                    };
                    println!("  {}  {:8} {}", type_str, file.size, file.name);
                }
            },
            Err(_) => {
                println!("Error: Cannot list directory '{}'", path);
            }
        }
    }
    
    fn cmd_cat(&self, args: &[&str]) {
        use crate::fs::vfs::VFS;
        
        if args.is_empty() {
            println!("Usage: cat <filename>");
            return;
        }
        
        let path = args[0];
        let vfs = VFS.lock();
        
        match vfs.read_file(path) {
            Ok(data) => {
                // Convert bytes to string and print
                let last_byte = data.last().copied();
                for byte in data {
                    if byte.is_ascii() {
                        print!("{}", byte as char);
                    }
                }
                if let Some(last) = last_byte {
                    if last != b'\n' {
                        println!();  // Add newline if file doesn't end with one
                    }
                }
            },
            Err(_) => {
                println!("Error: Cannot read file '{}'", path);
            }
        }
    }
    
    fn cmd_execute(&self, args: &[&str]) {
        if args.is_empty() {
            println!("Usage: exec <filename.exe>");
            return;
        }
        
        let filename = args[0];
        println!("Loading Windows executable: {}", filename);
        
        // For now, we'll load a hardcoded test program since we don't have a real filesystem yet
        // This is a minimal Windows PE executable that just returns
        let test_exe_data = create_test_exe();
        
        // Try to load and execute the PE file
        use crate::process::pe_loader::PeLoader;
        match PeLoader::load(&test_exe_data) {
            Ok(entry_point) => {
                println!("PE file loaded successfully!");
                println!("Entry point: 0x{:x}", entry_point);
                
                // For now, just print the entry point since process execution isn't fully implemented
                println!("Ready to execute at entry point 0x{:x}", entry_point);
                println!("Note: Full process execution is not yet implemented");
                
                // TODO: When process management is ready:
                // 1. Allocate memory for the process
                // 2. Load sections into memory
                // 3. Set up stack and heap
                // 4. Create process context
                // 5. Switch to user mode and jump to entry point
            }
            Err(e) => {
                println!("Failed to load PE file: {}", e);
            }
        }
    }
}

// Create a minimal test PE executable 
fn create_test_exe() -> Vec<u8> {
    use alloc::vec;
    
    // This is a minimal 64-bit PE that just returns
    // DOS header + stub
    let mut exe = vec![
        0x4D, 0x5A, // MZ signature
        0x90, 0x00, // bytes on last page
        0x03, 0x00, // pages
        0x00, 0x00, // relocations
        0x04, 0x00, // header size in paragraphs
        0x00, 0x00, // min alloc
        0xFF, 0xFF, // max alloc
        0x00, 0x00, // SS
        0xB8, 0x00, // SP
        0x00, 0x00, // checksum
        0x00, 0x00, // IP
        0x00, 0x00, // CS
        0x40, 0x00, // relocation table offset
        0x00, 0x00, // overlay
    ];
    
    // Reserved space
    exe.extend_from_slice(&[0u8; 32]);
    
    // PE offset at 0x3C
    exe.extend_from_slice(&[0x80, 0x00, 0x00, 0x00]); // PE header at 0x80
    
    // DOS stub
    exe.extend_from_slice(b"This program cannot be run in DOS mode.\r\r\n$");
    
    // Padding to PE header at 0x80
    while exe.len() < 0x80 {
        exe.push(0);
    }
    
    // PE signature
    exe.extend_from_slice(b"PE\0\0");
    
    // COFF header
    exe.extend_from_slice(&[
        0x64, 0x86, // Machine: x86_64
        0x01, 0x00, // NumberOfSections: 1
        0x00, 0x00, 0x00, 0x00, // TimeDateStamp
        0x00, 0x00, 0x00, 0x00, // PointerToSymbolTable
        0x00, 0x00, 0x00, 0x00, // NumberOfSymbols
        0xF0, 0x00, // SizeOfOptionalHeader
        0x22, 0x00, // Characteristics: EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE
    ]);
    
    // Optional header (PE32+)
    exe.extend_from_slice(&[
        0x0B, 0x02, // Magic: PE32+
        0x0E, 0x00, // MajorLinkerVersion
        0x00, 0x00, // MinorLinkerVersion
        0x00, 0x02, 0x00, 0x00, // SizeOfCode
        0x00, 0x00, 0x00, 0x00, // SizeOfInitializedData
        0x00, 0x00, 0x00, 0x00, // SizeOfUninitializedData
        0x00, 0x10, 0x00, 0x00, // AddressOfEntryPoint: 0x1000
        0x00, 0x10, 0x00, 0x00, // BaseOfCode: 0x1000
    ]);
    
    // ImageBase (8 bytes for PE32+)
    exe.extend_from_slice(&[0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00]); // 0x400000
    
    exe.extend_from_slice(&[
        0x00, 0x10, 0x00, 0x00, // SectionAlignment: 0x1000
        0x00, 0x02, 0x00, 0x00, // FileAlignment: 0x200
        0x06, 0x00, // MajorOperatingSystemVersion
        0x00, 0x00, // MinorOperatingSystemVersion
        0x00, 0x00, // MajorImageVersion
        0x00, 0x00, // MinorImageVersion
        0x06, 0x00, // MajorSubsystemVersion
        0x00, 0x00, // MinorSubsystemVersion
        0x00, 0x00, 0x00, 0x00, // Win32VersionValue
        0x00, 0x20, 0x00, 0x00, // SizeOfImage: 0x2000
        0x00, 0x02, 0x00, 0x00, // SizeOfHeaders: 0x200
        0x00, 0x00, 0x00, 0x00, // CheckSum
        0x03, 0x00, // Subsystem: CONSOLE
        0x00, 0x00, // DllCharacteristics
    ]);
    
    // Stack/heap sizes (8 bytes each for PE32+)
    exe.extend_from_slice(&[0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00]); // StackReserve
    exe.extend_from_slice(&[0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // StackCommit
    exe.extend_from_slice(&[0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00]); // HeapReserve
    exe.extend_from_slice(&[0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // HeapCommit
    
    exe.extend_from_slice(&[
        0x00, 0x00, 0x00, 0x00, // LoaderFlags
        0x10, 0x00, 0x00, 0x00, // NumberOfRvaAndSizes: 16
    ]);
    
    // Data directories (16 * 8 bytes)
    for _ in 0..16 {
        exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }
    
    // Section header (.text)
    exe.extend_from_slice(b".text\0\0\0"); // Name
    exe.extend_from_slice(&[
        0x00, 0x02, 0x00, 0x00, // VirtualSize
        0x00, 0x10, 0x00, 0x00, // VirtualAddress: 0x1000
        0x00, 0x02, 0x00, 0x00, // SizeOfRawData
        0x00, 0x02, 0x00, 0x00, // PointerToRawData: 0x200
        0x00, 0x00, 0x00, 0x00, // PointerToRelocations
        0x00, 0x00, 0x00, 0x00, // PointerToLinenumbers
        0x00, 0x00, // NumberOfRelocations
        0x00, 0x00, // NumberOfLinenumbers
        0x20, 0x00, 0x00, 0x60, // Characteristics: CODE | EXECUTE | READ
    ]);
    
    // Padding to section data at 0x200
    while exe.len() < 0x200 {
        exe.push(0);
    }
    
    // Code section - simple program that prints and returns
    // mov rax, 0 (return code)
    exe.extend_from_slice(&[0x48, 0xC7, 0xC0, 0x00, 0x00, 0x00, 0x00]); // mov rax, 0
    // ret
    exe.push(0xC3);
    
    // Pad to section size
    while exe.len() < 0x400 {
        exe.push(0);
    }
    
    exe
}

// Use a simpler static approach to avoid allocation issues
pub static SHELL: Mutex<Option<Shell>> = Mutex::new(None);

pub fn init() {
    // Initialize shell during boot
    let shell = Shell::new();
    shell.print_welcome();
    shell.print_prompt();
    *SHELL.lock() = Some(shell);
    crate::serial_println!("Shell initialized and ready for commands");
}

pub fn handle_keyboard_input(character: char) {
    if let Some(ref mut shell) = *SHELL.lock() {
        shell.handle_key(character);
    }
}