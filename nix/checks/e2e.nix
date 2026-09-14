{ pkgs, flake, ... }:
pkgs.testers.nixosTest {
  name = "horae-e2e";
  nodes.server = { config, lib, ... }: {
    imports = [ flake.nixosModules.horae ];
    services.horae.enable = true;
    services.horae.database.createLocally = true;
    systemd.services.horae.environment.DEV_LOGIN = "1";
    # Restart explicitly so crash recovery cannot race the test's assertions.
    systemd.services.horae.serviceConfig.Restart = lib.mkForce "no";
    # Put horae on PATH so the test script can call `horae seed`
    environment.systemPackages = [ config.services.horae.package ];
  };
  testScript = ''
    import json
    import shlex

    server.start()
    server.wait_for_unit("postgresql.service")
    server.wait_for_unit("horae.service")
    server.wait_for_open_port(3000)

    # Migrations must wait for the database ownership setup, not just the socket.
    for dependency in ("After", "Requires"):
        units = server.succeed(
            f"systemctl show horae.service --property={dependency} --value"
        ).split()
        assert "postgresql.target" in units, f"Missing PostgreSQL setup dependency: {dependency}"

    # Application migrations need ownership of horae, not permission to create
    # other databases. The separate sqlx test role still needs CREATEDB.
    def assert_no_createdb():
        allowed = server.succeed(
            "sudo -u postgres psql -At -c \"SELECT rolcreatedb FROM pg_roles WHERE rolname = 'horae'\""
        ).strip()
        assert allowed == "f", f"Service role has CREATEDB: {allowed}"
        server.fail("sudo -u horae createdb horae_forbidden_probe")

    assert_no_createdb()

    # Existing installations must lose the old privilege on setup as well.
    server.succeed("sudo -u postgres psql -v ON_ERROR_STOP=1 -c 'ALTER ROLE horae CREATEDB'")
    server.succeed("systemctl restart postgresql-setup.service")
    assert_no_createdb()
    server.succeed("systemctl restart horae.service")
    server.wait_for_unit("horae.service")
    server.wait_for_open_port(3000)

    # Health check
    server.succeed("curl -s http://localhost:3000/health | grep -q ok")

    # Backups are on by default with database.createLocally, and the unit
    # actually produces a dump rather than just existing.
    server.succeed("systemctl start postgresqlBackup-horae.service")
    server.succeed("test -s /var/backup/postgresql/horae.sql.gz")

    # Seed data — run as the horae user (DynamicUser in systemd creates it)
    # so the unix socket auth matches the DB owner.
    server.succeed("sudo -u horae DATABASE_URL=postgres:///horae horae seed")

    # Dev login: POST returns 303 redirect — don't use -f (fails on non-2xx)
    status = server.succeed(
      "curl -s -o /dev/null -w '%{http_code}' -X POST http://localhost:3000/auth/dev-login"
    ).strip()
    assert status == "303", f"Expected 303 redirect, got: {status}"

    # Full login flow with cookie jar (follow redirect)
    server.succeed(
      "curl -s -c /tmp/cookies.txt -L -X POST http://localhost:3000/auth/dev-login -o /dev/null"
    )

    # Harvest API: list time entries (session-authenticated)
    result = server.succeed(
      "curl -s -b /tmp/cookies.txt http://localhost:3000/harvest/v2/time_entries"
    )
    assert '"time_entries"' in result, f"Expected Harvest envelope, got: {result[:200]}"
    assert '"per_page"' in result, f"Missing pagination field in: {result[:200]}"

    # Exports are plain Axum routes under /api/, a prefix the login guard lets
    # through because everything else there is a server function that checks its
    # own session. They have to reject anonymous callers themselves.
    range = "from=2000-01-01&to=2100-01-01"
    status = server.succeed(
      "curl -s -o /dev/null -w '%{http_code}' "
      f"'http://localhost:3000/api/reports/export/csv?{range}'"
    ).strip()
    assert status == "401", f"Export served without a session: {status}"

    status = server.succeed(
      "curl -s -o /dev/null -w '%{http_code}' http://localhost:3000/api/projects/export/csv"
    ).strip()
    assert status == "401", f"Projects export served without a session: {status}"

    # …and still serve the data to a signed-in caller.
    csv = server.succeed(
      f"curl -s -b /tmp/cookies.txt 'http://localhost:3000/api/reports/export/csv?{range}'"
    )
    assert "Date,Project,Task" in csv, f"Authenticated export broken: {csv[:200]}"

    def sql(statement):
        return server.succeed(
            "sudo -u horae psql -d horae -At -v ON_ERROR_STOP=1 -c "
            + shlex.quote(statement)
        ).strip()

    def wait_sql(statement, expected):
        command = (
            "sudo -u horae psql -d horae -At -v ON_ERROR_STOP=1 -c "
            + shlex.quote(statement)
        )
        server.wait_until_succeeds(
            "test \"$(" + command + ")\" = " + shlex.quote(expected),
            timeout=90,
        )

    # The next batch blocks on an advisory lock held by a separate connection.
    # Waiting for the actual lock waiter proves the first batch has committed;
    # import speed cannot move the interruption before/after the target boundary.
    sql("""
        CREATE FUNCTION import_restart_gate() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            IF NEW.notes IN ('restart-term-500', 'restart-kill-500') THEN
                PERFORM pg_advisory_xact_lock(198, 500);
            END IF;
            RETURN NEW;
        END $$;
        CREATE TRIGGER import_restart_gate BEFORE INSERT ON time_entries
        FOR EACH ROW EXECUTE FUNCTION import_restart_gate();
    """)

    for termination in ("term", "kill"):
        prefix = f"restart-{termination}"
        header = "Date,Client,Project,Task,Hours,Email,Notes"
        records = [
            f"2024-01-15,Restart Client,Restart Project,Restart Task,1,admin@example.com,{prefix}-{i}"
            for i in range(1000)
        ]
        contents = chr(10).join([header, *records, ""])
        server.succeed("printf '%s' " + shlex.quote(contents) + " > /tmp/restart-import.csv")
        # pg_sleep only keeps the gate connection alive; synchronization below
        # observes PostgreSQL locks, not a guessed delay.
        server.succeed(
            "sudo -u horae env PGAPPNAME=horae-e2e-import-gate "
            "psql -d horae -v ON_ERROR_STOP=1 "
            "-c 'SELECT pg_advisory_lock(198, 500)' -c 'SELECT pg_sleep(300)' "
            "> /tmp/import-gate.log 2>&1 < /dev/null &"
        )
        wait_sql(
            "SELECT count(*) FROM pg_locks WHERE locktype = 'advisory' "
            "AND classid = 198 AND objid = 500 AND granted",
            "1",
        )

        def start_csv():
            response = server.succeed(
                "curl --fail-with-body -sS -b /tmp/cookies.txt "
                "-H 'X-Horae-Import: csv' --data-binary @/tmp/restart-import.csv "
                "http://localhost:3000/api/import/harvest/csv-job/Commit"
            )
            return json.loads(response)["id"]

        job_id = start_csv()
        wait_sql(
            "SELECT count(*) FROM pg_locks WHERE locktype = 'advisory' "
            "AND classid = 198 AND objid = 500 AND NOT granted",
            "1",
        )
        entries = f"SELECT count(*) FROM time_entries WHERE notes LIKE '{prefix}-%'"
        assert sql(entries) == "500", "First batch was not committed before interruption"
        assert sql(f"SELECT checkpoint IS NOT NULL FROM horae_jobs WHERE id = '{job_id}'") == "t"
        first_pid = server.succeed("systemctl show horae --property=MainPID --value").strip()
        assert first_pid != "0"
        if termination == "term":
            server.succeed("systemctl stop horae.service", timeout=45)
            assert server.succeed("systemctl show horae --property=Result --value").strip() == "success"
        else:
            server.succeed("systemctl kill --signal=SIGKILL --kill-whom=main horae.service")
        server.wait_until_succeeds("test ! -e /proc/" + first_pid, timeout=45)
        assert sql(entries) == "500", "An unconfirmed batch survived process termination"

        sql("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE application_name = 'horae-e2e-import-gate'")
        # Advance only this job's recovery deadline instead of waiting five
        # minutes for its lease. Recovery still runs in a new server process.
        sql(f"""
            UPDATE horae_jobs SET available_at = clock_timestamp(),
                lease_until = CASE WHEN status = 'running'
                    THEN clock_timestamp() - interval '1 second' ELSE lease_until END
            WHERE id = '{job_id}'
        """)
        server.succeed("systemctl restart horae.service")
        server.wait_for_open_port(3000)
        wait_sql(f"SELECT status FROM horae_jobs WHERE id = '{job_id}'", "succeeded")
        assert sql(entries) == "1000", "Recovered import lost or duplicated entries"
        assert sql(f"SELECT attempts FROM horae_jobs WHERE id = '{job_id}'") == "2"
        report = json.loads(sql(f"SELECT report FROM horae_jobs WHERE id = '{job_id}'"))
        assert report["summary"]["time_entries"]["created"] == 1000
        assert report["summary"]["time_entries"]["errored"] == 0
        assert sql(f"SELECT sum(minutes) FROM time_entries WHERE notes LIKE '{prefix}-%'") == "60000"

        repeated_id = start_csv()
        wait_sql(f"SELECT status FROM horae_jobs WHERE id = '{repeated_id}'", "succeeded")
        assert sql(entries) == "1000", "Reimport after recovery duplicated entries"
        report = json.loads(sql(f"SELECT report FROM horae_jobs WHERE id = '{repeated_id}'"))
        assert report["summary"]["time_entries"]["created"] == 0
        assert report["summary"]["time_entries"]["skipped"] == 1000
  '';
}
