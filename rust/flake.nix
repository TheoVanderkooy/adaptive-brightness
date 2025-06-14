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
    };
}
