{ pkgs ? import <nixpkgs> {
  config.allowUnfree = true;
} }:

with pkgs;

mkShell rec {
  nativeBuildInputs = [
  ];
  buildInputs = [
    # dioxus-cli
    just
    rustup
  ];
  LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
}
