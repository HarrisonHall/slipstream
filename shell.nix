{ pkgs ? import <nixpkgs> {
  config.allowUnfree = false;
} }:

with pkgs;

mkShell rec {
  nativeBuildInputs = [
  ];
  buildInputs = [
    just
    rustup
    patchelf
    cargo-zigbuild
  ];
  LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
}
