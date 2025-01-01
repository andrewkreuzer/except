{self, inputs, system, pkgs, meta, overlays}:
let
  pkgsCross = import inputs.nixpkgs {
    inherit system overlays;
    crossSystem = {
      config = "aarch64-unknown-linux-android";
      rust.rustcTarget = "aarch64-linux-android";
      androidSdkVersion = "33";
      androidNdkVersion = "26";
      useAndroidPrebuilt = true;
      useLLVM = true;
    };
  };
  androidRustPlatform = pkgsCross.makeRustPlatform {
    cargo = pkgs.rust-bin.nightly.latest.minimal.override {
      targets = ["aarch64-linux-android" "x86_64-unknown-linux-gnu"];
    };
    rustc = pkgs.rust-bin.nightly.latest.minimal.override {
      targets = ["aarch64-linux-android" "x86_64-unknown-linux-gnu"];
    };
  };
in
androidRustPlatform.buildRustPackage {
  inherit meta;
  pname = "except-android";
  version = (builtins.fromTOML
    (builtins.readFile ./Cargo.toml)).package.version;

  src = self;
  buildType = "release";
  buildFeatures = [];
  buildAndTestSubdir = "android";
  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  postUnpack = ''
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=${pkgs.androidenv.androidPkgs.ndk-bundle}/libexec/android-sdk/ndk-bundle/toolchains/llvm/prebuilt/${system}/bin/aarch64-linux-android35-clang
    export CC="${pkgs.androidenv.androidPkgs.ndk-bundle}/libexec/android-sdk/ndk-bundle/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang"
  '';

  nativeBuildInputs = [
    pkgs.androidenv.androidPkgs.ndk-bundle
  ];
}
