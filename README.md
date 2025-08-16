# Rust OS - ReactOS-Compatible Kernel in Rust

A Rust-based operating system kernel with Windows NT compatibility, inspired by ReactOS and Wine projects.

## Features

- **Windows PE/COFF Executable Support**: Can parse and load Windows .exe files
- **Interactive Shell**: Command-line interface with basic commands
- **Memory Management**: Heap allocation, paging, and virtual memory
- **x86_64 Architecture**: 64-bit kernel for modern systems
- **VGA Text Mode**: Console output support
- **Keyboard Input**: Both interrupt-driven and polling modes

## Quick Start

### Prerequisites

- Rust nightly toolchain
- QEMU for testing
- `bootimage` tool for creating bootable images

### Installation

```bash
# Install Rust nightly
rustup override set nightly

# Install bootimage
cargo install bootimage

# Install QEMU (macOS)
brew install qemu

# Install QEMU (Linux)
sudo apt-get install qemu-system-x86
```

### Building

```bash
# Build the kernel
cargo build --target x86_64-rust_os.json

# Create bootable image
cargo bootimage --target x86_64-rust_os.json
```

### Running

```bash
# Run in QEMU
./build-and-run.sh

# Or manually:
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rust_os/debug/bootimage-rust_kernel.bin \
    -serial mon:stdio \
    -display none
```

## Shell Commands

Once the kernel boots, you'll see the ReactOS shell prompt. Available commands:

- `help` - Show available commands
- `clear`/`cls` - Clear the screen
- `echo [text]` - Print text to screen
- `ver`/`version` - Show system version
- `mem`/`memory` - Show memory usage
- `ps`/`processes` - List running processes
- `exec`/`run [file.exe]` - Execute a Windows .exe file
- `test` - Run system tests
- `shutdown` - Shutdown the system
- `reboot` - Reboot the system

## Project Structure

```
rust-os/
├── kernel/
│   └── src/
│       ├── main.rs           # Kernel entry point
│       ├── cmd_shell.rs      # Interactive shell
│       ├── interrupts.rs     # Interrupt handling
│       ├── memory/           # Memory management
│       ├── process/          # Process management
│       │   └── pe_loader.rs  # Windows PE loader
│       ├── win32/           # Win32 API compatibility
│       └── nt/              # NT kernel compatibility
├── bootloader/              # Bootloader code
├── Cargo.toml              # Project configuration
├── x86_64-rust_os.json     # Target specification
└── build-and-run.sh        # Build and test script
```

## Current Status

### ✅ Working
- Kernel boots to interactive shell
- Basic memory management (1MB heap)
- PE/COFF file parsing
- Keyboard and serial input
- VGA text output

### 🚧 In Progress
- Full process execution
- Context switching
- Windows API implementation
- File system support

### 📋 Planned
- NTFS support
- Network stack
- Graphics mode
- USB support
- Sound drivers

## Windows Compatibility

The kernel includes a PE (Portable Executable) loader that can parse Windows .exe files. Currently, it can:
- Parse DOS and PE headers
- Identify x86_64 executables
- Extract entry points and sections
- Validate PE structure

Full execution support with Win32 API emulation is under development.

## Development

### Running Tests

```bash
# Run kernel tests
./test_kernel.sh

# Test PE execution
./test_exe.sh
```

### Debugging

```bash
# Run with GDB debugging
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rust_os/debug/bootimage-rust_kernel.bin \
    -serial mon:stdio \
    -s -S

# In another terminal:
gdb target/x86_64-rust_os/debug/rust_kernel
(gdb) target remote :1234
(gdb) continue
```

## Contributing

This project is part of the ReactOS to Rust migration effort. Contributions are welcome!

### Guidelines
- Follow Rust coding standards
- Maintain Windows API compatibility where applicable
- Document unsafe code blocks
- Add tests for new features

## Inspiration

This project draws inspiration from:
- [ReactOS](https://reactos.org/) - Windows-compatible operating system
- [Wine](https://www.winehq.org/) - Windows API implementation
- [Phil Opp's Blog OS](https://os.phil-opp.com/) - Rust OS development tutorials

## License

[To be determined - typically MIT or Apache 2.0 for Rust projects]

## Acknowledgments

- ReactOS team for the original C/C++ implementation
- Wine project for Windows API documentation
- Rust community for excellent OS development resources
