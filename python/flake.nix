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
        default = pkgs.mkShell
          {
            venvDir = ".venv";

            packages = with pkgs; [
              python3
              libusb1
              ddcutil
              python3Packages.pip
              python3Packages.venvShellHook
            ];

            # python ftdi library needs this explicitly
            # update dynamic lib path: https://discourse.nixos.org/t/what-is-the-nix-way-to-specify-ld-library-path/6407
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.libusb1 ];
          };
      });
    };
}
