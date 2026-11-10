# Harvest compatibility suite clients

Each client is an unmodified third-party Harvest API v2 client, pinned and built offline by
`nix/checks/harvest-compat.nix`, plus the smallest driver that runs one workflow through it.
Local patches, if a client ever needs one to change its base URL, live in `../patches/` and are
never sent upstream (D-8).

| Client | Language | Pin | Workflows | Status |
| --- | --- | --- | --- | --- |
| [becoded/go-harvest](https://github.com/becoded/go-harvest) | Go | `v0.0.0-20260118095129-1ac13afbeb71` (`go.sum`, `vendorHash`) | who am I, projects, time entries list and get | runs; reads through the dev-login session until tokens land |

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
