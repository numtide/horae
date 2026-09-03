{ pkgs, perSystem, ... }:
pkgs.mkShell {
  # Pull in the toolchain and build inputs from the horae package.
  inputsFrom = [ perSystem.self.default ];
  packages = with pkgs; [
    sqlx-cli
    postgresql
    process-compose
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
