{ flake, ... }:
{ config
, lib
, pkgs
, ...
}:

let
  cfg = config.services.horae;
in
{
  options.services.horae = {
    enable = lib.mkEnableOption "Horae time tracking server";

    package = lib.mkOption {
      type = lib.types.package;
      default = (pkgs.extend flake.overlays.shared-nixpkgs).horae;
      defaultText = lib.literalExpression "(pkgs.extend horae.overlays.shared-nixpkgs).horae";
      description = "The horae package to use.";
    };

    host = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "The host address the Horae server listens on.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 3000;
      description = "The TCP port the Horae server listens on.";
    };

    database = {
      url = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = ''
          PostgreSQL connection URL. When null and database.createLocally is true,
          defaults to a local Unix-socket connection: postgres:///horae.
        '';
      };

      createLocally = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          When true and database.url is null, configure a local PostgreSQL
          instance with a `horae` database owned by the `horae` service user.
        '';
      };

      backup = {
        enable = lib.mkOption {
          type = lib.types.bool;
          default = cfg.database.createLocally;
          defaultText = lib.literalExpression "config.services.horae.database.createLocally";
          description = ''
            Whether to take a periodic `pg_dump` of the Horae database via
            services.postgresqlBackup. Only applies to the local instance this
            module provisions; when the database lives elsewhere, back it up there.
          '';
        };

        location = lib.mkOption {
          type = lib.types.path;
          default = "/var/backup/postgresql";
          description = ''
            Directory the dumps are written to. The most recent dump is
            `horae.sql.gz`; the one before it is kept as `horae.sql.prev.gz`.
          '';
        };

        startAt = lib.mkOption {
          type = with lib.types; either (listOf str) str;
          default = "*-*-* 01:15:00";
          description = ''
            When to take the dump, in systemd.time(7) calendar format.
          '';
        };
      };
    };

    secretKeyFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = ''
        Path to a file containing environment variables with secrets
        (e.g. HORAE_OIDC_CLIENT_SECRET or HORAE_HARVEST_ENC_KEY). Loaded via
        systemd EnvironmentFile. Use a runtime path such as /run/secrets/horae-env
        and keep secret contents out of the Nix store. OIDC also requires
        HORAE_OIDC_ISSUER, HORAE_OIDC_CLIENT_ID, and HORAE_OIDC_REDIRECT_URL;
        these can be in the same file or systemd.services.horae.environment.
      '';
    };

    logLevel = lib.mkOption {
      # TODO: Should be an enum
      type = lib.types.str;
      default = "info";
      description = "Log verbosity level passed as HORAE_LOG.";
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Whether to open the firewall for the Horae port.";
    };
  };

  config = lib.mkIf cfg.enable {
    # Local PostgreSQL instance managed by this module.
    services.postgresql = lib.mkIf cfg.database.createLocally {
      enable = true;
      ensureDatabases = [ "horae" ];
      ensureUsers = [
        {
          name = "horae";
          ensureDBOwnership = true;
          ensureClauses = {
            # Explicit false also revokes the privilege on existing installs.
            # Only the separate development/test role needs CREATEDB.
            createdb = false;
            login = true;
          };
        }
      ];
    };

    # Nightly dumps of the database this module provisions. `createLocally`
    # invites the operator to treat durability as handled, so handle it.
    services.postgresqlBackup = lib.mkIf (cfg.database.createLocally && cfg.database.backup.enable) {
      enable = true;
      databases = [ "horae" ];
      inherit (cfg.database.backup) location startAt;
    };

    systemd.services.horae = {
      description = "Horae time tracking server";
      # The target waits for database/user provisioning as well as the server.
      after = [ "network.target" ] ++ lib.optionals cfg.database.createLocally [ "postgresql.target" ];
      requires = lib.optionals cfg.database.createLocally [ "postgresql.target" ];
      wantedBy = [ "multi-user.target" ];

      environment = {
        HORAE_LOG = cfg.logLevel;
        DATABASE_URL =
          if cfg.database.url != null then
            cfg.database.url
          else if cfg.database.createLocally then
            "postgres:///horae"
          else
            "postgres://localhost/horae";
      };

      serviceConfig =
        {
          ExecStartPre = "${cfg.package}/bin/horae migrate run";
          ExecStart = "${cfg.package}/bin/horae serve --host ${cfg.host} --port ${toString cfg.port}";
          DynamicUser = true;
          StateDirectory = "horae";
          # The Dioxus fullstack server looks for `public/` relative to its
          # working directory.  Point at the bin/ dir so it finds the bundled
          # WASM + assets that live at ${package}/bin/public/.
          WorkingDirectory = "${cfg.package}/bin";
          Restart = "on-failure";

          # Hardening
          NoNewPrivileges = true;
          ProtectSystem = "strict";
          ProtectHome = true;
          PrivateTmp = true;
        }
        // lib.optionalAttrs (cfg.secretKeyFile != null) {
          EnvironmentFile = cfg.secretKeyFile;
        };
    };

    networking.firewall.allowedTCPPorts = lib.mkIf cfg.openFirewall [ cfg.port ];
  };
}
