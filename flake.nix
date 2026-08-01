{
  description = "Rust Development Shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];

        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain =
          pkgs.rust-bin.nightly."2026-05-24".default.override {
              extensions = [
                "rust-src"
                "rust-analyzer"
              ];
            };
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustToolchain
            openssl
            gcc
            openjdk8
            pkg-config
          ];

          LD_LIBRARY_PATH = builtins.concatStringsSep ":" [
            (pkgs.lib.makeLibraryPath [ pkgs.openssl ])
            "target/debug"
          ];

          shellHook = ''
            alias java=$JAVA_HOME/bin/java
          '';
        };
      });
}
