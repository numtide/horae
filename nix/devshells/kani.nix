{ pkgs, perSystem, ... }:
# `nix develop .#kani -c cargo kani -p horae-core`. Kani brings its own nightly
# toolchain (about 1.4 GiB), so it stays out of the default shell.
pkgs.mkShell {
  packages = [ perSystem.self.kani ];
  meta.platforms = perSystem.self.kani.meta.platforms;
}
