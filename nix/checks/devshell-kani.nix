{ pkgs, ... }:
# Replaces blueprint's automatic check of the Kani dev shell, for the reasons
# in pkgs-kani.nix: CI neither builds nor evaluates Kani.
pkgs.emptyFile
