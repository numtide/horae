{ pkgs, perSystem, ... }:
# Replaces blueprint's automatic check of the Kani package, which would build
# Kani and its nightly toolchain on every CI run. This only evaluates it.
pkgs.runCommand "pkgs-kani-evaluates" { } ''
  echo ${builtins.unsafeDiscardStringContext perSystem.self.kani.drvPath} > $out
''
