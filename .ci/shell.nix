let
  deps = import ../nix/deps.nix { targets = [ "x86_64-unknown-linux-musl" ]; };
  nixpkgs = import <nixpkgs> {};
in
  with nixpkgs;
  nixpkgs.mkShell {
    name = "ci_shell";
    buildInputs = deps;

    RUST_BACKTRACE = 1;
  }
