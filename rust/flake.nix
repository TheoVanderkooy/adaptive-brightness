{
  description = "Dev flake for adaptive brightness project";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.*.tar.gz";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      supportedSystems = [ "x86_64-linux" ];
      forEachSupportedSystem =
        f:
        nixpkgs.lib.genAttrs supportedSystems (
          system:
          f {
            pkgs = import nixpkgs {
              inherit system;
              overlays = [
                rust-overlay.overlays.default
                self.overlays.default
              ];
            };
          }
        );
    in
    {
      overlays.default = final: prev: {
        rustToolchain =
          let
            rust = prev.rust-bin;
          in
          if builtins.pathExists ./rust-toolchain.toml then
            rust.fromRustupToolchainFile ./rust-toolchain.toml
          else if builtins.pathExists ./rust-toolchain then
            rust.fromRustupToolchainFile ./rust-toolchain
          else
            rust.stable.latest.default.override {
              extensions = [
                "rust-src"
                "rustfmt"
              ];
            };
      };

      devShells = forEachSupportedSystem (
        { pkgs }:
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              rustToolchain
              cargo
              rustc
              rustfmt
              pre-commit
              rustPackages.clippy
              rust-analyzer

              libftdi1
              ddcutil
              libusb1
            ];

            nativeBuildInputs = with pkgs; [
              pkg-config
            ];

            env = {
              # For rust-analyzer
              RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
            };
          };
        }
      );

      packages = forEachSupportedSystem (
        { pkgs }:
        {
          default =
            let
              manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
            in
            pkgs.rustPlatform.buildRustPackage {
              pname = manifest.name;
              version = manifest.version;
              cargoLock = {
                lockFile = ./Cargo.lock;
                # this is to avoid needing to specify outputHashes for my own libraries pulled from github
                allowBuiltinFetchGit = true;
              };
              src = pkgs.lib.cleanSource ./.;

              buildInputs = with pkgs; [
                libftdi1
                ddcutil
                libusb1
              ];

              nativeBuildInputs = with pkgs; [
                pkg-config
              ];
            };
        }
      );

      formatter.x86_64-linux = nixpkgs.legacyPackages.x86_64-linux.nixfmt-rfc-style;
    };
}
