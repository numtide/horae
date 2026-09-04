# Formatting as a flake check, so anything that builds the flake's checks
# verifies it too — `nix fmt -- --ci` in a workflow step is invisible to
# builders that only evaluate the flake.
{ inputs, pkgs, ... }:
(inputs.treefmt.lib.evalModule pkgs ../treefmt.nix).config.build.check inputs.self
