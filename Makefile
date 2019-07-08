.PHONY: clean run run-serial check
LLVM-OBJCOPY ?= llvm-objcopy

default: kernel8.img

check: test format clippy

kernel8.debug.img: cargo-build-debug
	$(LLVM-OBJCOPY) target/aarch64-unknown-none/debug/sneka --strip-all -O binary kernel8.debug.img

cargo-build-debug:
	cargo xbuild --target aarch64-unknown-none

kernel8.img: cargo-build
	$(LLVM-OBJCOPY) target/aarch64-unknown-none/release/sneka --strip-all -O binary kernel8.img

cargo-build:
	cargo xbuild --release --target aarch64-unknown-none

clippy:
	cargo clippy -- --version
	cargo xclippy --target aarch64-unknown-none -- -D warnings

test:
	cargo test

format:
	cargo fmt --version
	cargo fmt -- --check

clean:
	rm -f kernel8.img
	cargo clean

run: kernel8.img
	qemu-system-aarch64 -M raspi3 -kernel kernel8.img -serial stdio

run-debug-int: kernel8.img
	qemu-system-aarch64 -M raspi3 -kernel kernel8.img -serial stdio -d int

run-debug: kernel8.debug.img
	lldb -ex "platform remote-gdb-server | qemu-system-aarch64 -M raspi3 -kernel kernel8.debug.img -S -gdb stdio" target/aarch64-unknown-none/debug/sneka

run-serial: kernel8.img
	qemu-system-aarch64 -M raspi3 -kernel kernel8.img -nographic
