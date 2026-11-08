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
    cargo clippy -p horae --features server --all-targets -- -D warnings -W clippy::perf
    cargo clippy -p horae-dioxus-lint --all-targets -- -D warnings -W clippy::perf
    runHook postBuild
  '';

  installPhase = "touch $out";
  doCheck = false;
  dontFixup = true;
})
