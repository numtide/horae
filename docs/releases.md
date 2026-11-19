# Versions and releases

Administrators see the running version above the account menu: the server's and the browser
bundle's. A release build shows its tag (`v0.2.0`); any other build shows `dev` and the commit it
came from (`dev · 1e1971e`, with `-dirty` for uncommitted changes). The server's version is asked
again on every page change. When the two differ, the browser still runs a bundle from before a
deploy, and the panel offers a reload.

## Cutting a release

A build is a release only when it is given the tag. To cut one:

1. Bump `version` in `crates/horae/Cargo.toml` (for example to `0.2.0`), run `cargo check` so
   `Cargo.lock` follows, and merge that.

1. Tag the merge commit `v0.2.0`, push the tag, and create the GitHub release from it.

1. Build from the tagged checkout with the tag passed in:

   ```sh
   nix build --impure --expr '(builtins.getFlake "git+file://${toString ./.}").packages.${builtins.currentSystem}.default.override { releaseTag = "v0.2.0"; }'
   ```

   The container image takes that package in place of the default one:

   ```sh
   nix build --impure --expr 'let p = (builtins.getFlake "git+file://${toString ./.}").packages.${builtins.currentSystem}; in p.oci-image.override { horae = p.default.override { releaseTag = "v0.2.0"; }; }'
   ```

   The `git+file://` reference matters: it reads only tracked files and gives the build its
   commit. In a git worktree, a bare path is copied whole into the store, `target/` included.

   Outside Nix, set `HORAE_RELEASE_TAG=v0.2.0` for `dx build`. The build fails unless the tag is
   `v` followed by the crate version, and Nix also refuses a checkout with uncommitted changes.
