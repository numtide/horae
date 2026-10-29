# Tasks: OCI Container Image

**Spec**: [oci-image.spec.md](oci-image.spec.md) · **Plan**: [oci-image.plan.md](oci-image.plan.md)

- [x] T001 [US1] Build the image in `nix/oci-image.nix` and expose `packages.<system>.oci-image` in `flake.nix`
- [x] T002 [US1] Verify `packages.x86_64-linux.default` keeps its output path, and `aarch64-linux` evaluates the image
- [x] T003 [P] [US1] [US2] Daemonless format, config and content check in `nix/checks/oci-image.nix`
- [ ] T004 [US1] [US2] Runtime check in `nix/checks/oci-image-runtime.nix`: podman load, CLI subcommands, `/health`, assets, non-root, read-only root, no dev-login bypass (written; not yet run)
- [x] T005 [P] Document build, load, run and what is not published in `README.md`
- [ ] T006 `nix fmt` and `nix flake check`
