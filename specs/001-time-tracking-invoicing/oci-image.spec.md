# Feature Specification: OCI Container Image

**Parent**: [001 Time Tracking & Invoicing](spec.md), success criterion SC-008 (self-hosting)
**Feature Branch**: `feat/nix-oci-image`
**Created**: 2026-10-29
**Status**: Draft
**Plan**: [oci-image.plan.md](oci-image.plan.md) · **Tasks**: [oci-image.tasks.md](oci-image.tasks.md)

This is a packaging increment to SC-008. It adds a second way to self-host, beside the NixOS
module, and changes no parent requirement.

## User Scenarios & Testing

### User Story 1 - Run Horae from a container image (Priority: P1)

An operator without NixOS builds the image from the flake, loads it into a container runtime,
points it at their own PostgreSQL and reaches the sign-in page.

**Independent Test**: load the built archive with podman, run it against a disposable database,
and fetch `/health` and the bundled assets.

**Acceptance Scenarios**:

1. **Given** the flake, **When** the operator runs `nix build .#oci-image`, **Then** the result is
   an OCI image-layout archive for the host architecture (x86_64-linux or aarch64-linux).
1. **Given** the loaded image and a reachable PostgreSQL, **When** it starts with only
   `DATABASE_URL`, **Then** it applies migrations, listens on port 3000 and serves the WASM, JS and
   CSS bundle.
1. **Given** the image, **When** the operator runs `migrate run`, `init …` or `user list` in it,
   **Then** the CLI behaves as the installed binary does.

### User Story 2 - Safe by default (Priority: P1)

**Acceptance Scenarios**:

1. **Given** the image's defaults, **When** it runs, **Then** the process is not root and the root
   filesystem can be read-only with only `/tmp` writable.
1. **Given** `DEV_LOGIN=1` passed to the container, **When** it starts, **Then** the server refuses,
   because the bind address is not loopback.
1. **Given** no session, **When** a caller posts to `/auth/dev-login` or reads the Harvest API,
   **Then** nobody is signed in and the API answers 401.

### Edge Cases

- OIDC and Harvest need outbound HTTPS: the image carries CA certificates.
- Docker loads the image after a documented `skopeo` conversion to a Docker archive.

## Requirements

- **FR-001**: The flake MUST expose `packages.<system>.oci-image` for x86_64-linux and
  aarch64-linux, built from the existing default package without recompiling it.
- **FR-002**: The output MUST be an OCI image layout archive (OCI index, manifest, config and layer
  media types), not a Docker-only archive presented as OCI.
- **FR-003**: The image MUST run `horae serve` by default on `0.0.0.0:3000`, with the binary as an
  absolute entrypoint, no shell, and the asset directory as working directory.
- **FR-004**: The image MUST run as uid/gid 65534 and include CA certificates.
- **FR-005**: The image MUST NOT contain secrets, database data, `DEV_LOGIN` or a database server.
- **FR-006**: Automated checks MUST verify the archive's format and config without a daemon, and the
  running image against a disposable PostgreSQL in a VM.
- **FR-007**: The README MUST document build, load and run commands, and that no image is published.

## Success Criteria

- **SC-001**: `nix flake check` builds the image and passes both image checks.
- **SC-002**: The default package derivation is unchanged by this feature.

## Assumptions

- No registry or push workflow until the project owner chooses one.
- PostgreSQL is provided by the operator, outside the container.
