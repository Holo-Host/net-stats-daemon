{
  description = "A very basic flake";

  inputs = { nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable"; };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      flake = {
        # Put your original flake attributes here.
      };

      systems = [ "x86_64-linux" "aarch64-linux" ];

      perSystem = { pkgs, system, lib, ... }: {
        devShells.default =
          pkgs.mkShell { packages = [ pkgs.nixfmt pkgs.cargo ]; };
      };
    };
}
