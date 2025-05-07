{
  description = "Nix for axelar-relayer-core";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs@{ flake-parts, fenix, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        ./nix/devshells.nix
      ];
      systems = [ "x86_64-linux" "aarch64-darwin" ];

      perSystem = { system, pkgs, ... }: {
        # per-system attributes can be defined here. the self' and inputs'
        # module parameters provide easy access to attributes of the same
        # system.
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [ fenix.overlays.default ];
        };

        formatter = pkgs.nixpkgs-fmt;
      };
    };
}
