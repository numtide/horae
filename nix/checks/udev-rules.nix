{ pkgs, ... }:
pkgs.testers.nixosTest {
  name = "udev-rule-reload";
  nodes.server = { lib, ... }: {
    systemd.package = import ../test-systemd.nix { inherit pkgs; };
    virtualisation.useNixStoreImage = true;
    virtualisation.writableStore = true;
    systemd.settings.Manager.DefaultDeviceTimeoutSec = lib.mkForce 900;
    systemd.services.systemd-udevd.environment.SYSTEMD_LOG_LEVEL = "debug";
  };
  testScript = ''
    server.start()
    retry(lambda _: "connecting to host..." in server.get_console_log(), timeout_seconds=900)
    server.succeed("udevadm settle --timeout=900")
    journal = server.succeed("journalctl -b -u systemd-udevd.service --no-pager -o cat")
    assert "Reading rules file:" in journal, "Expected udev debug diagnostics"
    reloads = journal.count("Udev rules need reloading")
    assert reloads <= 1, f"Unchanged rules repeatedly reloaded: {reloads}"

    # New and modified rules must still be noticed without an explicit reload.
    server.succeed("mkdir -p /run/udev/rules.d")
    for value in ("first", "second"):
        rule = f'KERNEL=="hvc0", ENV{{HORAE_CONSOLE_RELOAD_PROBE}}="{value}"'
        server.succeed(f"printf '%s\\n' '{rule}' > /run/udev/rules.d/99-console-probe.rules")
        server.wait_until_succeeds(
            "udevadm trigger --action=change --settle /dev/hvc0 && "
            "udevadm info --query=property --name=/dev/hvc0 | "
            f"grep -Fx 'HORAE_CONSOLE_RELOAD_PROBE={value}'",
            timeout=60,
        )
  '';
}
