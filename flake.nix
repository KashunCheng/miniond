{
  description = "Alternative implementation of the Emulab client-side programs";

  inputs = {
    mars-std.url = "github:mars-research/mars-std";
    mars-std.inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
    mars-std.inputs.rust-overlay.url = "github:oxalica/rust-overlay/ce117f3e0de8262be8cd324ee6357775228687cf";
  };

  outputs = { self, mars-std, ... }: let
    # System types to support.
    supportedSystems = [ "x86_64-linux" ];

    # Rust nightly version.
    nightlyVersion = "2023-08-01";
  in mars-std.lib.eachSystem supportedSystems (system: let
    pkgs = mars-std.legacyPackages.${system};
    lib = pkgs.lib;

    rustNightly = pkgs.rust-bin.nightly.${nightlyVersion}.default.override {
      extensions = [ "rust-src" "rust-analyzer-preview" ];
      targets = [
        # FIXME: Other platforms
        "x86_64-unknown-linux-gnu"
        "x86_64-unknown-linux-musl"
      ];
    };

    buildMiniond = pkgs: let
      rustPlatform = pkgs.makeRustPlatform {
        rustc = rustNightly;
        cargo = rustNightly;
      };
    in rustPlatform.buildRustPackage {
      pname = "miniond";
      version = "0.1.0";

      src = lib.cleanSourceWith {
        filter = name: type: !(builtins.elem (baseNameOf name) ["target" "miniond"]);
        src = lib.cleanSourceWith {
          filter = lib.cleanSourceFilter;
          src = ./.;
        };
      };

      cargoLock.lockFile = ./Cargo.lock;
    };
  in rec {
    packages = {
      miniond = buildMiniond pkgs;
      miniondStatic = buildMiniond pkgs.pkgsStatic;
    };
    defaultPackage = self.packages.${system}.miniond;

    devShell = pkgs.mkShell {
      nativeBuildInputs = with pkgs; [
        rustNightly
      ];
    };
  }) // {
    nixosConfigurations.testSystem = mars-std.inputs.nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./nixos/emulab.nix
        ./nixos/miniond.nix

        ({ pkgs, lib, ... }: {
          nixpkgs.overlays = [
            (_: super: {
              miniond = self.packages."x86_64-linux".miniond;
            })
          ];
          hardware.emulab.enable = true;
          boot.isContainer = true;
        })
      ];
    };
  };
}
