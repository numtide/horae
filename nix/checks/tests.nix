{ perSystem, pkgs, ... }:
perSystem.self.default.overrideAttrs (old: {
  pname = "horae-tests";
  nativeBuildInputs = old.nativeBuildInputs ++ [
    pkgs.cargo-nextest
    pkgs.postgresql
  ];

  # Compile-time sqlx macros (query!, query_as!, …) resolve from the .sqlx/
  # cache, not from the database started below — so a cache that has drifted
  # from the migrations fails a test here instead of being papered over by a
  # live connection.
  SQLX_OFFLINE = "true";

  buildPhase = ''
    runHook preBuild
    export HOME=$(mktemp -d)

    # horae-core: pure domain tests — no database required.
    cargo nextest run -p horae-core

    # The rest of the suite is #[sqlx::test]: every test creates a throwaway
    # database — so the role needs CREATEDB, which `postgres` has as a
    # superuser — and applies crates/horae/migrations into it itself. Unix
    # socket only, no TCP, because the build sandbox has no network. The
    # cluster dies with the build directory, so durability buys nothing.
    export PGDATA=$(mktemp -d)
    initdb -D "$PGDATA" --no-locale --encoding=UTF8 -U postgres
    pg_ctl -D "$PGDATA" -l "$PGDATA/log" \
      -o "--unix_socket_directories=$PGDATA --listen_addresses= \
          -c fsync=off -c synchronous_commit=off -c full_page_writes=off" start
    createdb -h "$PGDATA" -U postgres horae
    export DATABASE_URL="postgres://postgres@localhost/horae?host=$PGDATA"

    # cargo test rather than nextest: the integration tests are #[serial], and
    # serial_test's lock is in-process. nextest runs each test in its own
    # process, which would leave them running concurrently regardless.
    cargo test -p horae --features server

    pg_ctl -D "$PGDATA" stop
    runHook postBuild
  '';

  installPhase = "touch $out";
  doCheck = false;
  dontFixup = true;
})
