{ pkgs ? import <nixpkgs> {
  config.allowUnfree = false;
} }:

with pkgs;

let  overrides = (builtins.fromTOML (builtins.readFile ./rust-toolchain.toml));
in

mkShell rec {
  nativeBuildInputs = [
  ];
  buildInputs = [
    just
    rustup
    rustPlatform.bindgenHook
    patchelf
    cargo-zigbuild
  ];
  LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
  RUSTC_VERSION = overrides.toolchain.channel;
  # https://github.com/rust-lang/rust-bindgen#environment-variables
  shellHook = ''
    export PATH="''${CARGO_HOME:-~/.cargo}/bin":"$PATH"
    export PATH="''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-${stdenv.hostPlatform.rust.rustcTarget}/bin":"$PATH"
  '';
}
