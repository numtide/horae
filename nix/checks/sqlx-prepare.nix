{ perSystem, pkgs, ... }:
perSystem.self.default.overrideAttrs (old: {
  pname = "horae-sqlx-prepare";
  cargoArtifacts = old.passthru.checkArtifacts;
  # No commit in the version label, so a commit that changes no source keeps this cached.
  HORAE_GIT_REV = "";
  nativeBuildInputs = old.nativeBuildInputs ++ [
    pkgs.postgresql
    pkgs.sqlx-cli
  ];

  # Override SQLX_OFFLINE so macros connect to our temporary database.
  SQLX_OFFLINE = "";

  buildPhase = ''
    runHook preBuild
    export HOME=$(mktemp -d)

    # Start a temporary PostgreSQL instance (unix socket only, no TCP).
    export PGDATA=$(mktemp -d)
    initdb -D "$PGDATA" --no-locale --encoding=UTF8 -U postgres
    pg_ctl -D "$PGDATA" -l "$PGDATA/log" \
      -o "--unix_socket_directories=$PGDATA --listen_addresses=" start
    createdb -h "$PGDATA" -U postgres horae
    export DATABASE_URL="postgres://postgres@localhost/horae?host=$PGDATA"

    # Apply migrations then verify the .sqlx/ cache is up-to-date.
    cargo sqlx migrate run --source crates/horae/migrations
    # Same package set as checkArtifacts, so its dependency artifacts are reused.
    cargo sqlx prepare --workspace --check -- \
      --workspace --exclude horae-dioxus-lint --features server --all-targets

    pg_ctl -D "$PGDATA" stop
    runHook postBuild
  '';

  installPhase = "touch $out";
  doCheck = false;
  dontFixup = true;
})
