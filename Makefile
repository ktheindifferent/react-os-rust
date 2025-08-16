.PHONY: all build run clean qemu debug

CARGO = cargo
QEMU = qemu-system-x86_64
KERNEL = target/x86_64-rust_os/debug/rust_kernel
BOOTIMAGE = target/x86_64-rust_os/debug/bootimage-rust_kernel.bin

all: build

build:
	@echo "Building Rust OS kernel..."
	cd kernel && $(CARGO) build
	@echo "Creating bootable image..."
	cargo bootimage --target x86_64-rust_os.json

run: build qemu

qemu:
	@echo "Booting Rust OS in QEMU..."
	$(QEMU) -drive format=raw,file=$(BOOTIMAGE) \
		-serial stdio \
		-display none \
		-m 512M \
		-cpu qemu64,+x2apic

debug: build
	@echo "Starting QEMU with debugging enabled..."
	$(QEMU) -drive format=raw,file=$(BOOTIMAGE) \
		-serial stdio \
		-m 512M \
		-cpu qemu64,+x2apic \
		-s -S &
	@echo "Connect with: gdb -ex 'target remote :1234' $(KERNEL)"

clean:
	@echo "Cleaning build artifacts..."
	$(CARGO) clean
	rm -rf target/

install-deps:
	@echo "Installing dependencies..."
	cargo install bootimage
	rustup component add rust-src llvm-tools-preview

test:
	@echo "Running tests..."
	cd kernel && $(CARGO) test