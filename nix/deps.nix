{ targets ? [ "x86_64-unknown-linux-gnu" ] }:

let
  moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  nixpkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
  rustWithComponents = (nixpkgs.rustChannelOf { date = "2019-10-15"; channel = "nightly"; }).rust.override {
        extensions = [
          "rust-src"
          "clippy-preview"
          "rustfmt-preview"
        ];
        inherit targets;
      };
in
  with nixpkgs;
  [
    rustWithComponents
    gnumake
    cacert
    cargo-xbuild
    llvm_8
  ]
