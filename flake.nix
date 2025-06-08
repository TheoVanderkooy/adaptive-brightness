{
  description = "Dev flake for adaptive brightness project";

  inputs.nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1";

  outputs = inputs:
    let
      supportedSystems = [ "x86_64-linux" ];
      forEachSupportedSystem = f: inputs.nixpkgs.lib.genAttrs supportedSystems (system: f {
        pkgs = import inputs.nixpkgs { inherit system; };
      });
    in
    {
      devShells = forEachSupportedSystem ({ pkgs }: {
        default = pkgs.mkShell.override
          {
            # Override stdenv in order to change compiler:
            # stdenv = pkgs.clangStdenv;
          }
          {
            packages = with pkgs; [
              # C
              clang-tools
              cmake
              codespell
              conan
              cppcheck
              doxygen
              gtest
              lcov
              vcpkg
              vcpkg-tool

              # needed for python libraries
              libusb1

              # rust:
              cargo rustc rustfmt pre-commit rustPackages.clippy

              # for ddcthingy
              ddcutil
            ] ++ (if system == "aarch64-darwin" then [ ] else [ gdb ]);

            nativeBuildInputs = with pkgs; [
              pkg-config
            ];

            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.libusb1 pkgs.ddcutil ];

            # update dynamic lib path: https://discourse.nixos.org/t/what-is-the-nix-way-to-specify-ld-library-path/6407
          };
      });
    };
}
