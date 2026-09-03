{ pkgs, perSystem, ... }:
let
  # Apply pending migrations against the running database, then rebuild the app
  # so sqlx re-checks its query macros against the new schema. Migrations are
  # already applied on every `process-compose up`; this is for applying one
  # without restarting the stack.
  migrate = pkgs.writeShellApplication {
    name = "horae-migrate";
    runtimeInputs = with pkgs; [ sqlx-cli process-compose git ];
    text = ''
      cd "$(git rev-parse --show-toplevel)"
      sqlx migrate run --source crates/horae/migrations

      # A new migration changes what the query macros compile against, so the app
      # needs a rebuild rather than a restart — which is what restarting dx does.
      if process-compose process list >/dev/null 2>&1; then
        echo "rebuilding the app against the new schema"
        process-compose process restart app
      fi
    '';
  };
in
pkgs.mkShell {
  # Pull in the toolchain and build inputs from the horae package.
  inputsFrom = [ perSystem.self.default ];
  packages = with pkgs; [
    sqlx-cli
    postgresql
    process-compose
    pgweb
    migrate
    nil
    typst
  ];
  shellHook = ''
    # Serves both local database options: the `process-compose up` stack and
    # the port forwarded by `nix run .#postgres`.
    export DATABASE_URL=postgres://horae@127.0.0.1:5432/horae

    # process-compose serves its REST API on :8080 by default, which is the
    # port dx serve uses. Move it so both can run.
    export PC_PORT_NUM=8088
  '';
}
