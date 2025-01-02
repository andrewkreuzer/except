{pkgs, treefmt-nix}:
let
  treefmt = treefmt-nix.lib.evalModule pkgs {
    projectRootFile = "flake.nix";
    programs.nixpkgs-fmt.enable = true;
  };
in
pkgs.mkShell {
  LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
  ANDROID_NDK_HOME = pkgs.androidenv.androidPkgs.ndk-bundle;
  formatter = treefmt.config.build.wrapper;
  buildInputs = with pkgs; [
    pkg-config
    openssl
    pam
    rustPlatform.bindgenHook
    pkgs.androidenv.androidPkgs.ndk-bundle
    (rust-bin.nightly.latest.default.override {
      targets = ["aarch64-linux-android" "x86_64-unknown-linux-gnu"];
    })
    rust-analyzer
    nixd
  ];
}
