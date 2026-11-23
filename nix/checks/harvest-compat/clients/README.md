# Harvest compatibility suite clients

Each client is an unmodified third-party Harvest API v2 client, pinned and built offline by
`nix/checks/harvest-compat.nix`, plus the smallest driver that runs one workflow through it.
Local patches, if a client ever needs one to change its base URL, live in `../patches/` and are
never sent upstream (D-8).

| Client | Language | Pin | Workflows | Status |
| --- | --- | --- | --- | --- |
| [becoded/go-harvest](https://github.com/becoded/go-harvest) | Go | `v0.0.0-20260118095129-1ac13afbeb71` (`go.sum`, `vendorHash`) | who am I, projects, time entries list and get, company, my project assignments; log, edit, start and end times, start (a second start stops the first), stop, restart, delete; sync of five lists by page and by link, then since an edit | runs; reads and writes with a personal access token and `Harvest-Account-Id` |

Not yet added (research R-10; each is added with the step that first needs it, or dropped here
with the reason):

- minutes-harvest (Rust)
- hrvst-cli (Node)
- harvest-tui
- dedene/harvest-cli

## Updating a pin

```sh
cd nix/checks/harvest-compat/clients/go-harvest
go get github.com/becoded/go-harvest@<commit> && go mod tidy
```

Then set `vendorHash` in `harvest-compat.nix` to `pkgs.lib.fakeHash`, build the check once and
copy the hash Nix reports.
