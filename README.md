# 🖤 Salmiak

[![CircleCI](https://circleci.com/gh/abbec/salmiak.svg?style=svg)](https://circleci.com/gh/abbec/salmiak)
[![Matrix](https://img.shields.io/matrix/salmiak:matrix.org.svg?label=Chat%20%40%20%23salmiak%3Amatrix.org&style=for-the-badge)](https://matrix.to/#/#salmiak:matrix.org)

A lightweight OS in library form to create games for the Pi
like in the old days (a'la Nintendo 64).

`salmiak` is the OS library for interacting with the Raspberry Pi 3
hardware and `sneka` is the proof of concept game.

## 🔧 Environment Setup (Using Nix)

Using Nix means that you have a repeatable setup that works on any (Unix) machine and you do not
have to modify the state of the machine since the environment of the nix shell is dropped when it is
closed. Neat, huh!?

- Install [Nix](https://nixos.org/nix/)
- run `nix-shell` in the root of the repo and you will get a shell with everything needed (inluding
  QEMU).

## 🔧 Environment Setup (Not using Nix)

- Install [Rust](https://rustup.rs/) and the nightly toolchain
- Add src (to nightly): `$ rustup component add rust-src`
- Install cargo xbuild: `$ cargo install cargo-xbuild`
- Make sure you have `llvm-objcopy`. If the executable is not called that, you can provide a
  variable to the make command below like `make LLVM-OBJCOPY=my-objcopy`.

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
