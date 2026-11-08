//! This build's version label. The server and the WASM front-end compile it
//! from the same source and environment, so a cached older bundle shows a
//! different label from the server it talks to.

/// Set only for a release build; see "Versions and releases" in README.md.
const RELEASE_TAG: Option<&str> = option_env!("HORAE_RELEASE_TAG");

const _: () = assert!(
    horae_core::version::release_tag_matches(RELEASE_TAG, env!("CARGO_PKG_VERSION")),
    "HORAE_RELEASE_TAG must be `v` followed by the crate version"
);

/// `v0.2.0` for a release, else `dev · <commit>`. The commit comes from the
/// Nix flake or, outside Nix, from git in `build.rs`.
pub fn current() -> String {
    horae_core::version::label(RELEASE_TAG, option_env!("HORAE_GIT_REV"))
}
