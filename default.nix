{self, pkgs, meta, rustPlatform}:
rustPlatform.buildRustPackage {
  inherit meta;
  pname = "except";
  version = (builtins.fromTOML
    (builtins.readFile ./Cargo.toml)).package.version;

  src = self;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  buildType = "release";
  buildFeatures = [];

  nativeBuildInputs = with pkgs; [
    pkg-config
    openssl
    pam
    rustPlatform.bindgenHook
  ];

  buildInputs = with pkgs; [
    openssl
    pkg-config
    pam
    rustPlatform.bindgenHook
  ];

}
