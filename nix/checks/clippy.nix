{ perSystem, ... }:
perSystem.self.default.overrideAttrs (old: {
  pname = "horae-clippy";
  cargoArtifacts = old.passthru.checkArtifacts;
  # No commit in the version label, so a commit that changes no source keeps this cached.
  HORAE_GIT_REV = "";

  # Let compile-time sqlx macros (query!, query_as!, …) resolve from the
  # .sqlx/ cache instead of requiring a live database connection.
  SQLX_OFFLINE = "true";

  buildPhase = ''
    runHook preBuild
    export HOME=$(mktemp -d)
    cargo clippy -p horae-core --all-targets -- -D warnings -W clippy::perf
    cargo clippy -p horae-types --all-targets -- -D warnings -W clippy::perf
    # The browser bundle compiles horae-types too.
    cargo clippy -p horae-types --target wasm32-unknown-unknown -- -D warnings
    cargo clippy -p horae-i18n --all-targets -- -D warnings -W clippy::perf
    # The browser bundle compiles the catalog too.
    cargo clippy -p horae-i18n --target wasm32-unknown-unknown -- -D warnings
    cargo clippy -p horae-harvest --all-targets -- -D warnings -W clippy::perf
    cargo clippy -p horae-server --all-targets -- -D warnings -W clippy::perf
    cargo clippy -p horae-api --all-targets -- -D warnings -W clippy::perf
    cargo clippy -p horae-harvest-api --all-targets -- -D warnings -W clippy::perf
    cargo clippy -p horae --features server --all-targets -- -D warnings -W clippy::perf
    cargo clippy -p horae-dioxus-lint --all-targets -- -D warnings -W clippy::perf
    runHook postBuild
  '';

  installPhase = "touch $out";
  doCheck = false;
  dontFixup = true;
})
