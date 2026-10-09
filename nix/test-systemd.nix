{ pkgs }:
# systemd 260.2 records original rule paths but checks resolved parent paths.
# NixOS symlinks its rules directory, causing unchanged rules to reload for
# every batch of events. Keep this correction limited to the test guests.
pkgs.systemd.overrideAttrs (old: {
  postPatch = (old.postPatch or "") + ''
    substituteInPlace src/udev/udev-rules.c \
      --replace-fail 'hashmap_put_stats_by_path(&rules->stats_by_path, c->original_path, &c->st)' \
                     'hashmap_put_stats_by_path(&rules->stats_by_path, c->result, &c->st)'
  '';
})
