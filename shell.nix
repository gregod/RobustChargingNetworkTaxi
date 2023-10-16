let
  oxalica_overlay = import (builtins.fetchTarball
    "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");
  nixpkgs = import <nixpkgs> { overlays = [ oxalica_overlay ]; };

in with nixpkgs;


pkgs.mkShell {
  name = "rust-env";
  nativeBuildInputs = [ gurobi ];
  buildInputs = [
    (rust-bin.nightly.latest.default.override { extensions = [ "rust-src" ]; })
  ];

  # Set Environment Variables
  RUST_BACKTRACE = 1;

  # this is required to compile gurobi-sys
  GUROBI_HOME = "${gurobi}";

}
