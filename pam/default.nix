{self, pkgs, meta, rustPlatform}:
rustPlatform.buildRustPackage {
  inherit meta;
  name = "except-pam-module";
  version = (builtins.fromTOML
    (builtins.readFile ./Cargo.toml)).package.version;

  src = self;
  buildAndTestSubdir = "pam";
  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = with pkgs; [
    pkg-config
    openssl
    pam
    rustPlatform.bindgenHook
  ];

  buildInputs = with pkgs; [
    pkg-config
    openssl
    pam
    rustPlatform.bindgenHook
  ];
}

