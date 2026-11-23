{ pkgs, flake, system, ... }:
# Replaces blueprint's automatic check of the Kani dev shell, which would build
# Kani and its nightly toolchain on every CI run. This only evaluates it.
pkgs.runCommand "devshell-kani-evaluates" { } ''
  echo ${builtins.unsafeDiscardStringContext flake.devShells.${system}.kani.drvPath} > $out
''
