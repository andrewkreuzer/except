{
  description = "A circuit breaker implementation in Rust";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, treefmt-nix, ... }@inputs:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config.android_sdk.accept_license = true;
          config.allowUnfree = true;
        };

        rustPlatform = pkgs.makeRustPlatform {
          cargo = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
          rustc = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
        };

        meta = {
          description = "authenetication module for PAM";
          homepage = "https://github.com/andrewkreuzer/except";
          license = with pkgs.lib.licenses; [ mit unlicense ];
          maintainers = [{
            name = "Andrew Kreuzer";
            email = "me@andrewkreuzer.com";
            github = "andrewkreuzer";
            githubId = 17596952;
          }];
        };
      in
      {
        packages.default = import ./. { inherit self pkgs meta rustPlatform; };
        packages.pamModule = import ./pam { inherit self pkgs meta rustPlatform; };
        packages.androidLib = import ./android { inherit self inputs system pkgs meta overlays; };

        imports = [ treefmt-nix.flakeModule ];
        devShells.default = import ./shell.nix { inherit pkgs treefmt-nix; };
      }
    );
}
