# Implementation Plan: OCI Container Image

**Spec**: [oci-image.spec.md](oci-image.spec.md) · **Parent**: [plan.md](plan.md) · **Tasks**: [oci-image.tasks.md](oci-image.tasks.md)

## Summary

`pkgs.dockerTools.buildLayeredImage` assembles the image from the existing default package, CA
certificates and `fakeNss`; `skopeo copy docker-archive:… oci-archive:…` converts it to an OCI image
layout inside the same derivation. Two checks cover it: a daemonless content check and a NixOS VM
test running the image under podman.

## Technical Context

- **Tooling**: nixpkgs `dockerTools`, `skopeo`, `testers.nixosTest`; no new flake input, no
  `flake.lock` change.
- **Platforms**: x86_64-linux and aarch64-linux, the flake's Linux systems.
- **Production code**: unchanged. The bind address comes from the existing `HORAE_HOST`/`HORAE_PORT`
  environment variables of `serve`; `DEV_LOGIN` is refused off loopback by the existing
  `check_dev_login_bind`.

## Constitution Check

- I Exactness, II Domain Purity, III Single Datastore, IV Mutations: not affected.
- V Reproducible Builds: the image is a flake output, built and checked by `nix flake check`.
- Local development bypass off by default: the image never sets `DEV_LOGIN`, and the VM test
  proves it cannot be turned on with the image's bind.

## Project Structure

```text
flake.nix                         # packages.<system>.oci-image
nix/oci-image.nix                 # the image (nixpkgs-style, callPackage)
nix/checks/oci-image.nix          # daemonless format/config/content check
nix/checks/oci-image-runtime.nix  # podman + disposable PostgreSQL in a VM
README.md                         # Self-hosting → Container image
```

## Complexity Tracking

| Choice | Why | Simpler alternative rejected because |
|---|---|---|
| `nix/oci-image.nix` wired in `flake.nix`, not `nix/packages/oci-image.nix` | `nix/package.nix` filters `.nix`/`.md` files but keeps directories, so a new `nix/packages/` directory entered the source and changed the default package hash (verified: `8m06…` → `cpc8…`), forcing a full Rust rebuild | Changing the source filter is out of scope for a packaging change. Blueprint's automatic `pkgs-oci-image` check is lost, so `nix/checks/oci-image.nix` builds the image explicitly |
| This child spec lives as files in `specs/001-…/` | A new `specs/018-…/` directory would change the default package hash the same way | — |
| Docker archive converted by skopeo | `dockerTools` writes only the Docker archive format | Shipping the Docker archive as "OCI" would misstate the format |
| VM test reuses `nix/test-systemd.nix`, `useNixStoreImage`, `writableStore` and 900 s waits | Same TCG/ARM timing as the existing e2e checks | Without them the earlier CI failures could return |
