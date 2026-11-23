{ pkgs, ... }:
# Replaces blueprint's automatic check of the Kani package. CI must not build
# Kani and its nightly toolchain, and cannot even evaluate them: the toolchain
# comes from fenix's manifest, an import from derivation that the CI refuses.
# Kani is used locally through `nix develop .#kani`.
pkgs.emptyFile
