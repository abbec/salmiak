let
  deps = import ./nix/deps.nix {};
  nixpkgs = import <nixpkgs> {};
in
  with nixpkgs;
  nixpkgs.mkShell {
    name = "dev_shell";
    buildInputs = deps ++ [ qemu lldb ];

    RUST_BACKTRACE = 1;
  }
