let
  moz_overlay = import (builtins.fetchTarball
    "https://github.com/mozilla/nixpkgs-mozilla/archive/9b11a87c0cc54e308fa83aac5b4ee1816d5418a2.tar.gz");
  nixpkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
in with nixpkgs;
let
  dlopen-libs = with xorg; [
    vulkan-loader
    libX11
    libXcursor
    libXrandr
    libXi
    libxkbcommon
  ];
in mkShell.override {
  stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;
} {
  nativeBuildInputs = with pkgs; [
    (rustChannels.stable.rust.override {
      extensions = [ "rust-src" "rust-analysis" ];
    })
    pkg-config
    alsa-lib
          pkgs.cargo-bloat
          pkgs.cargo-unused-features
          pkgs.cargo-watch
          pkgs.cargo-sort
          pkgs.cargo-machete
          pkgs.cargo-depgraph
          pkgs.cargo-limit
          pkgs.cargo-flamegraph
          pkgs.cargo-insta
          pkgs.pre-commit
  ];
  shellHook = ''
    export RUST_BACKTRACE=1
    export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${lib.makeLibraryPath dlopen-libs}"
    export RUST_LOG=game=trace,scrap3=trace
    pre-commit install
  '';
}
