# Contributing to Rust OS

Thank you for your interest in contributing to the Rust OS project! This guide will help you get started.

## Getting Started

1. **Fork the repository** and clone your fork locally
2. **Set up the development environment** following the README instructions
3. **Create a new branch** for your feature or bug fix
4. **Make your changes** following our coding standards
5. **Test your changes** thoroughly
6. **Submit a pull request** with a clear description

## Development Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/rust-os.git
cd rust-os

# Set up Rust nightly
rustup override set nightly
rustup component add rust-src llvm-tools-preview

# Install dependencies
cargo install bootimage

# Build and test
./build-and-run.sh
```

## Code Style Guidelines

### Rust Code
- Follow standard Rust formatting with `rustfmt`
- Use meaningful variable and function names
- Document public APIs with doc comments
- Keep functions focused and small
- Prefer safe Rust; document all `unsafe` blocks

### Commit Messages
- Use present tense ("Add feature" not "Added feature")
- Keep the first line under 50 characters
- Provide detailed explanation in the body if needed
- Reference issues with "Fixes #123" or "Relates to #456"

Example:
```
Add PE executable loader for Windows compatibility

- Implement DOS header parsing
- Add PE/COFF header validation
- Support x86_64 executables only
- Add basic section loading

Fixes #42
```

## Testing

Before submitting a PR, ensure:

1. **Code compiles** without warnings:
   ```bash
   cargo build --target x86_64-rust_os.json
   ```

2. **Kernel boots** successfully:
   ```bash
   ./test_kernel.sh
   ```

3. **Tests pass** (if applicable):
   ```bash
   cargo test --target x86_64-rust_os.json
   ```

## Areas for Contribution

### High Priority
- **Process Execution**: Complete PE loader execution
- **System Calls**: Implement Windows NT system calls
- **Memory Management**: Virtual memory, demand paging
- **File Systems**: FAT32/NTFS support

### Medium Priority
- **Device Drivers**: Storage, network, USB
- **Graphics**: VESA/VBE support, window manager
- **Networking**: TCP/IP stack
- **Win32 APIs**: kernel32, user32, gdi32

### Good First Issues
- **Shell Commands**: Add new shell commands
- **Documentation**: Improve code documentation
- **Tests**: Add unit and integration tests
- **Bug Fixes**: Fix known issues from TODO.md

## Architecture Guidelines

### Module Organization
```
kernel/src/
├── main.rs           # Entry point
├── process/          # Process management
├── memory/           # Memory management
├── fs/               # File systems
├── drivers/          # Device drivers
├── win32/            # Win32 subsystem
└── nt/               # NT kernel layer
```

### Adding New Features

1. **Plan the feature**: Discuss in an issue first
2. **Design the API**: Keep it simple and Rust-idiomatic
3. **Implement incrementally**: Small, reviewable PRs
4. **Add tests**: Unit tests where possible
5. **Document thoroughly**: Both code and usage

### Windows Compatibility

When implementing Windows compatibility:
- Refer to Wine and ReactOS documentation
- Maintain ABI compatibility where needed
- Use Windows data structures and constants
- Test with real Windows executables when possible

## Pull Request Process

1. **Update documentation** if you changed APIs
2. **Add tests** for new functionality
3. **Ensure CI passes** (when available)
4. **Request review** from maintainers
5. **Address feedback** promptly
6. **Squash commits** if requested

## Debugging Tips

### QEMU Debugging
```bash
# Start QEMU with GDB server
qemu-system-x86_64 \
    -drive format=raw,file=target/x86_64-rust_os/debug/bootimage-rust_kernel.bin \
    -serial mon:stdio \
    -s -S

# Connect with GDB
gdb target/x86_64-rust_os/debug/rust_kernel
(gdb) target remote :1234
(gdb) break rust_main
(gdb) continue
```

### Serial Output
Use `serial_println!` for debug output:
```rust
serial_println!("Debug: value = {:?}", value);
```

### Panic Handling
Check serial output for panic messages and backtraces.

## Community

- **Issues**: Report bugs and request features
- **Discussions**: Ask questions and share ideas
- **Discord/IRC**: [To be added]

## License

By contributing, you agree that your contributions will be licensed under the same license as the project.

## Recognition

Contributors will be acknowledged in:
- Git history
- CREDITS file
- Release notes

Thank you for helping make Rust OS better!