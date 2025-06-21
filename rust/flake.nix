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
        default = pkgs.mkShell          {
          packages = with pkgs; [
            cargo rustc rustfmt pre-commit rustPackages.clippy rust-analyzer

            libftdi1
            ddcutil
            libusb1
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
        };
      });

      packages = forEachSupportedSystem ({ pkgs }: {
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
      });
    };
}
