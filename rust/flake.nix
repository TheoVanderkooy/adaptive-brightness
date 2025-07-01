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
              clang
              llvmPackages.bintools
            ];

            nativeBuildInputs = with pkgs; [
              pkg-config
            ];

            env = {
              # For rust-analyzer
              RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";

              # Required by bindgen
              LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ pkgs.llvmPackages_latest.libclang.lib ];
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
                clang
                llvmPackages.bintools
              ];

              nativeBuildInputs = with pkgs; [
                pkg-config
              ];
              preBuild = ''
                # https://hoverbear.org/blog/rust-bindgen-in-nix/
                # From: https://github.com/NixOS/nixpkgs/blob/1fab95f5190d087e66a3502481e34e15d62090aa/pkgs/applications/networking/browsers/firefox/common.nix#L247-L253
                # Set C flags for Rust's bindgen program. Unlike ordinary C
                # compilation, bindgen does not invoke $CC directly. Instead it
                # uses LLVM's libclang. To make sure all necessary flags are
                # included we need to look in a few places.
                export BINDGEN_EXTRA_CLANG_ARGS="$(< ${pkgs.stdenv.cc}/nix-support/libc-crt1-cflags) \
                  $(< ${pkgs.stdenv.cc}/nix-support/libc-cflags) \
                  $(< ${pkgs.stdenv.cc}/nix-support/cc-cflags) \
                  $(< ${pkgs.stdenv.cc}/nix-support/libcxx-cxxflags) \
                  ${pkgs.lib.optionalString pkgs.stdenv.cc.isClang "-idirafter ${pkgs.stdenv.cc.cc}/lib/clang/${pkgs.lib.getVersion pkgs.stdenv.cc.cc}/include"} \
                  ${pkgs.lib.optionalString pkgs.stdenv.cc.isGNU "-isystem ${pkgs.stdenv.cc.cc}/include/c++/${pkgs.lib.getVersion pkgs.stdenv.cc.cc} -isystem ${pkgs.stdenv.cc.cc}/include/c++/${pkgs.lib.getVersion pkgs.stdenv.cc.cc}/${pkgs.stdenv.hostPlatform.config} -idirafter ${pkgs.stdenv.cc.cc}/lib/gcc/${pkgs.stdenv.hostPlatform.config}/${pkgs.lib.getVersion pkgs.stdenv.cc.cc}/include"} \
                "
              '';

              env = {
                # Required by bindgen
                LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ pkgs.llvmPackages_latest.libclang.lib ];
              };
            };
        }
      );

      formatter.x86_64-linux = nixpkgs.legacyPackages.x86_64-linux.nixfmt-rfc-style;
    };
}
