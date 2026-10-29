{ pkgs, flake, system, ... }:
# Runs the OCI image under podman against a disposable PostgreSQL in the VM,
# the way an operator would: the database lives outside the container and
# configuration arrives as environment variables.
let
  image = flake.packages.${system}.oci-image;
  inherit (flake.packages.${system}.default) version;
in
pkgs.testers.nixosTest {
  name = "horae-oci-image-runtime";
  nodes.machine = { lib, ... }: {
    systemd.package = import ../test-systemd.nix { inherit pkgs; };
    # Keep guest executable reads off 9p when the builder falls back to TCG.
    virtualisation.useNixStoreImage = true;
    # Preserve the writable overlay needed by register-nix-paths.service.
    virtualisation.writableStore = true;
    virtualisation.memorySize = 2048;
    virtualisation.diskSize = 4096;
    # Under TCG, coldplug can exceed the test harness's five-minute device limit.
    systemd.settings.Manager.DefaultDeviceTimeoutSec = lib.mkForce 900;
    # initdb can exceed PostgreSQL's 120-second startup limit under ARM TCG.
    systemd.services.postgresql.serviceConfig.TimeoutStartSec = 900;

    virtualisation.podman.enable = true;

    services.postgresql = {
      enable = true;
      ensureDatabases = [ "horae" ];
      ensureUsers = [{ name = "horae"; ensureDBOwnership = true; }];
      authentication = lib.mkOverride 10 ''
        local all all              trust
        host  all all 127.0.0.1/32 trust
      '';
    };
  };

  testScript = ''
    image = "localhost/horae:${version}"
    database = "-e DATABASE_URL=postgres://horae@127.0.0.1:5432/horae"
    # Host networking reaches the VM's PostgreSQL on loopback. The root
    # filesystem is read-only; only /tmp is writable.
    run = f"podman run --rm --network=host --read-only --tmpfs /tmp {database}"

    machine.start()
    # TCG boots on ARM can exceed the driver's fixed five-minute shell timeout.
    retry(lambda _: "connecting to host..." in machine.get_console_log(), timeout_seconds=900)
    machine.wait_for_unit("postgresql.target", timeout=900)

    # The archive is OCI image layout, and podman loads it as such.
    machine.succeed("podman load -i ${image}")
    machine.succeed(f"podman image exists {image}")

    # CLI subcommands run from the same image.
    machine.succeed(f"{run} {image} migrate run")
    assert "No users found." in machine.succeed(f"{run} {image} user list")
    machine.succeed(
        f"{run} {image} init --org-name 'OCI Test' "
        "--admin-email admin@example.com --admin-name Admin"
    )
    assert "admin@example.com" in machine.succeed(f"{run} {image} user list")

    # The development login bypass refuses the container's non-loopback bind.
    status, output = machine.execute(f"{run} -e DEV_LOGIN=1 {image} 2>&1")
    assert status != 0, f"Server started with DEV_LOGIN=1: {output}"
    assert "DEV_LOGIN=1 grants an admin session" in output, output

    machine.succeed(
        "podman run -d --name horae --network=host --read-only --tmpfs /tmp "
        f"{database} {image}"
    )
    try:
        machine.wait_for_open_port(3000, timeout=300)
        machine.wait_until_succeeds("curl -fsS http://127.0.0.1:3000/health | grep -q ok", timeout=120)

        # The default user is unprivileged.
        pid = machine.succeed("podman inspect --format '{{.State.Pid}}' horae").strip()
        uid = machine.succeed(f"stat -c %u /proc/{pid}").strip()
        assert uid == "65534", f"Server runs as uid {uid}"

        # Every bundled asset is served byte for byte.
        root = machine.succeed(f"podman image mount {image}").strip()
        public = f"{root}/nix/store/" + machine.succeed(
            f"ls {root}/nix/store | grep -- '-horae-${version}$'"
        ).strip() + "/bin/public/assets"
        assets = machine.succeed(f"ls {public}").split()
        for suffix in (".wasm", ".js", ".css"):
            assert any(a.endswith(suffix) for a in assets), f"No {suffix} asset: {assets}"
        for asset in assets:
            served = machine.succeed(f"curl -fsS http://127.0.0.1:3000/assets/{asset} | sha256sum").split()[0]
            bundled = machine.succeed(f"sha256sum {public}/{asset}").split()[0]
            assert served == bundled, f"Asset {asset} served differently"
        wasm_type = machine.succeed(
            "curl -fsS -o /dev/null -w '%{content_type}' "
            f"http://127.0.0.1:3000/assets/{next(a for a in assets if a.endswith('.wasm'))}"
        )
        assert wasm_type == "application/wasm", f"WASM content type: {wasm_type}"
        machine.succeed(f"podman image unmount {image}")

        # No authentication bypass: the dev login form is absent, its route
        # signs nobody in, and session-protected APIs reject anonymous callers.
        login = machine.succeed("curl -fsS http://127.0.0.1:3000/auth/login")
        assert "/auth/dev-login" not in login, "Login page offers the dev bypass"
        headers = machine.succeed(
            "curl -sS -o /dev/null -D - -X POST http://127.0.0.1:3000/auth/dev-login"
        )
        assert " 303" not in headers.splitlines()[0], f"Dev login redirected: {headers}"
        status = machine.succeed(
            "curl -sS -o /dev/null -w '%{http_code}' http://127.0.0.1:3000/harvest/v2/time_entries"
        ).strip()
        assert status == "401", f"Harvest API served without a session: {status}"
    finally:
        print(machine.execute("podman logs horae 2>&1 | tail -n 80")[1])
  '';
}
