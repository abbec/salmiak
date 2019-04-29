.PHONY: clean run run-serial check

default: kernel8.img

check: test format clippy

ifeq ($(DEBUG),1)
kernel8.img: cargo-build
	cd sneka && cargo objcopy -- ../target/aarch64-unknown-none/debug/sneka --strip-all -O binary ../kernel8.img

cargo-build:
	cargo xbuild --target aarch64-unknown-none
else
kernel8.img: cargo-build
	cd sneka && cargo objcopy -- ../target/aarch64-unknown-none/release/sneka --strip-all -O binary ../kernel8.img

cargo-build:
	cargo xbuild --release --target aarch64-unknown-none
endif

clippy:
	cargo clippy -- --version
	cargo xclippy --target aarch64-unknown-none -- -D warnings

test:
	cargo test

format:
	cargo fmt -- --version
	cargo fmt -- --check

clean:
	rm -f kernel8.img
	cargo clean

run: kernel8.img
	qemu-system-aarch64 -M raspi3 -kernel kernel8.img -serial stdio

run-debug-int: kernel8.img
	qemu-system-aarch64 -M raspi3 -kernel kernel8.img -serial stdio -d int

run-serial: kernel8.img
	qemu-system-aarch64 -M raspi3 -kernel kernel8.img -nographic
