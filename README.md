# 🖤 Salmiak

[![CircleCI](https://circleci.com/gh/abbec/salmiak.svg?style=svg)](https://circleci.com/gh/abbec/salmiak)

A lightweight OS in library form to create games for the Pi
like in the old days (a'la Nintendo 64).

`salmiak` is the OS library for interacting with the Raspberry Pi 3
hardware and `sneka` is the proof of concept game.

## 🔧 Environment Setup

- Install [Rust](https://rustup.rs/) and the nightly toolchain
- Add src and llvm-tools (to nightly): `$ rustup component add rust-src llvm-tools-preview`
- Install cargo binutils and cargo xbuild: `$ cargo install cargo-xbuild cargo-binutils`

## 🚜 Building the Code

To build, just issue

	$ make

## 🏃 Running the Code in the Emulator

To run the build code in QEMU (assuming you have it installed), issue

	$ make run

## 🧪 Running the Tests

Tests are run on the host platform by issuing

	$ make test

which is an alias for `cargo test`.

## 🚩 Running clippy

[Clippy](https://github.com/rust-lang/rust-clippy) is a linter to check for common mistakes in the
code. It can be run by issuing

	$ make clippy

This requires clippy to be installed which can be done for your toolchain with `rustup component add
clippy`.

## 📏 Running rustfmt
[rustfmt](https://github.com/rust-lang/rustfmt) is a tool for formatting Rust code according to
style guidelines. A check for format issues can be run on the codebase by issuing

	$ make format


This requires rustfmt to be installed which can be done for your toolchain with `rustup component add
rustfmt`.

## Running all checks

To run all checks (test + format + clippy) issue

	$ make check

This, in addition to just `make` is what the CI checks.
