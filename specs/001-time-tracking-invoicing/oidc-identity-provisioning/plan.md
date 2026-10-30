# Implementation Plan: Operator-provisioned OIDC identity binding

**Branch**: `feat/oidc-identity-provisioning` | **Date**: 2026-10-30 | **Base**: `c71a2756` | **Spec**: [spec.md](spec.md)

## Summary

Record the provider issuer with every OIDC binding, resolve sign-ins only by the
issuer and subject pair, and add one local operator command,
`horae user link-oidc`, that binds a provider identity to an existing active account
(or completes that account's own legacy binding) under a row lock. `user list`
gains the account ID. One nullable column and one CHECK constraint; no new
dependency, UI or permission change.

- Evidence and decisions: [research.md](research.md)
- States and transitions: [data-model.md](data-model.md)
- Command output, exit codes and resolution order: [contracts/link-oidc-cli.md](contracts/link-oidc-cli.md)
- Operator flow, upgrade step, validation and requirement → proof: [quickstart.md](quickstart.md)

## Technical Context

- **Language/Version**: Rust 2024 (toolchain pinned by the flake).
- **Primary Dependencies**: existing only: `openidconnect` 3.5 (`IssuerUrl`, claim
  validation), `sqlx`, `clap`, `anyhow`. No new Rust or production dependency, no
  `flake.lock` change. A test-only crypto package (for example Python `PyJWT` +
  `cryptography`) may be used inside the NixOS OIDC check.
- **Storage**: PostgreSQL. Migration `0049_oidc_issuer.sql` adds
  `users.oidc_issuer text NULL` and `CHECK (oidc_issuer IS NULL OR oidc_subject IS NOT NULL)`.
  `UNIQUE (oidc_subject)` stays. `.sqlx` regenerated with
  `--features server --all-targets` in an own disposable migrated database.
- **Testing**: unit tests for validation and the pure link decision;
  `#[sqlx::test]` tests for link outcomes, concurrency, resolution and plugin
  privileges; a binary test for CLI output and exit codes; the NixOS `e2e-oidc`
  check for the packaged command with Dex, plus the absent-claim full flow through a
  test-only Dex fixture patch (research D6).
- **Target Platform**: Linux server (the CLI is the server binary).
- **Project Type**: web service with an operator CLI.
- **Performance Goals**: none new. Sign-in keeps one indexed lookup by subject.
- **Constraints**: exact issuer and subject bytes; no session or role writes from
  the command; the new code adds the issuer to no `models::User`, page query or
  plugin payload, and the command never prints or logs issuer or subject. Existing
  exposures (subject in `models::User`, issuer text in an existing validation-error
  log) stay as documented in research.md.
- **Scale/Scope**: single organization, one configured provider.
- **Touched code**:
  - `crates/horae/migrations/0049_oidc_issuer.sql` (new).
  - `crates/horae/src/auth/oidc.rs`: `Identity.issuer`; pair lookup; email-path
    update sets the issuer; `mod link;`.
  - `crates/horae/src/auth/oidc/link.rs` (new): validation, pure `decide`, `link`.
  - `crates/horae/src/auth/oidc/tests/linking.rs`: issuer-aware cases.
  - `crates/horae/src/cli.rs`: `UserAction::LinkOidc`; parse tests.
  - `crates/horae/src/main.rs`: dispatch and exit handling; `user list` ID column.
  - `crates/horae/src/plugin/database.rs`: `oidc_issuer` privilege guard and test.
  - `crates/horae/tests/` : a CLI binary test for link output and exit codes.
  - `nix/checks/e2e-oidc.nix`: repeat link after the verified sign-in; the
    absent-claim scenario with a test-only `services.dex.package` override and a
    dedicated `horae-omit-email-verified` client (D6). Test-only Python
    `PyJWT` + `cryptography` in the VM if used for signature checks.
  - Docs: README (operator flow, upgrade step), AGENTS.md CLI list,
    `001/contracts/plugin-interface.md` (issuer excluded), `001/data-model.md`
    (field list), `015/contracts/permission-state.md` (identity-writer row).
- **Not touched**: `init.rs`, `user create`, `models::User` and its consumers, pages,
  CSS, server functions, permission code, session layer, 014 settings, Shipyard.

## Constitution Check

Constitution 3.0.0, checked before Phase 0 and again after Phase 1.

| Principle | Status |
|-----------|--------|
| I. Exactness | N/A. No durations or money. |
| II. Domain purity | Pass. The binding rules are a pure function unit-tested without I/O, beside the existing pure `resolve_identity` in `auth`. Principle II's list (durations, money, totals, entry/invoice transitions) does not cover identity, so `horae-core` is unchanged. |
| III. Single datastore | Pass. PostgreSQL only; no new table; `users` already has `org_id` and a UUID v7 key. |
| IV. Mutations through server functions | Pass under the 3.0.0 exception: `link-oidc` is the named local operator identity binding. The plan keeps every limit: existing active target, no replacement, same-subject legacy completion only, no accounts/sessions/provider users/roles/other fields, not reachable over HTTP, UI or plugins. The sign-in callback's existing binding write is a separate deviation, justified in Complexity Tracking. |
| V. Reproducible builds | `nix fmt`, offline `.sqlx` build and `nix flake check` on the exact head before merge, in a granted slot. |

After design: the exception is used as written. This check covers only the paths
this feature touches; it makes no whole-application compliance claim and waives no
other mutation.

## Project Structure

```text
specs/001-time-tracking-invoicing/oidc-identity-provisioning/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md  tasks.md
├── contracts/link-oidc-cli.md
└── checklists/requirements.md
```

```text
crates/horae/
├── migrations/0049_oidc_issuer.sql
├── src/auth/oidc.rs
├── src/auth/oidc/link.rs
├── src/auth/oidc/tests/linking.rs
├── src/cli.rs
├── src/main.rs
├── src/plugin/database.rs
└── tests/oidc_link_cli.rs
nix/checks/e2e-oidc.nix
```

**Structure Decision**: The command's logic lives in `auth/oidc/link.rs`, a child of
the existing `oidc.rs` root (the `foo.rs` + `foo/` layout), because it is the
operator side of the same identity contract. `main.rs` only parses, calls and maps
the outcome to output and exit code.

## Spec Kit procedure record

- Constitution: amended to 3.0.0 (Principle IV exception) with a Sync Impact Report;
  templates and runtime docs checked. `.specify/extensions.yml` is absent, so no
  hooks ran at any step.
- Scripts ran with `SPECIFY_INIT_DIR` set to a scratch copy of `.specify/` and
  `SPECIFY_FEATURE_DIRECTORY` set to this directory, so the tracked
  `.specify/feature.json` (016) was never written. Ran
  `check-prerequisites.sh --json --paths-only` (specify, clarify),
  `setup-plan.sh --json` (copied the plan template here) and
  `setup-tasks.sh`/`check-prerequisites.sh` for tasks and analyze.
- The spec was created from `spec-template.md` by copy (the specify procedure does
  that itself; `create-new-feature.sh` was not used because the child directory was
  chosen in advance and it would number a top-level `specs/NNN-*`).
- The plan's agent-context update step has no script in this checkout; not run,
  nothing substituted.
- Clarify asked the owner nothing new: the answers are technical decisions taken in
  contract review within the approved outcome.

## Decisions accepted in contract review

- D4: when `HORAE_OIDC_ISSUER` is set and non-empty, the issuer must equal it.
- D5: exit 1 for every rejection and error, 2 for usage.
- D6: test-only Dex fixture patch, pending test evidence. Still to confirm while
  implementing: the Dex package attribute, that `services.dex.package` applies, and
  a dry run presented before the heavy check.
- Still to confirm on the migrated schema: `users_oidc_subject_key` as the 23505
  constraint name (T001).
- Under review: the Complexity Tracking entry below.

## Analyze record

`/speckit-analyze` (read-only) ran after tasks: 20/20 requirements covered, no
placeholders, no duplications. Findings: C1 (above), I1 (FR-012/SC-006 overclaimed
that the subject never reaches the browser; it already does through `models::User`,
which this feature leaves alone), T1 (outcome naming). The author then fixed I1 and
T1 in the spec and recorded C1 here.

## Gates

- No product code, build or database work until the contract review approves it.
- Builds and test runs only in a granted slot (one heavy job, `max-jobs`/
  `build-jobs` 1, no incremental, 4 GiB watchdog).
- No push or PR without an explicit grant; independent reviews before merge.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Principle IV, existing: `/auth/callback` writes `users.oidc_subject` outside a server function (since 001). Narrow addition here: the same conditional UPDATE also writes `oidc_issuer`, from the same validated token. Nothing else in that write changes. | The write is the owner-approved automatic binding of an active, unbound account by verified email, which this feature preserves. It runs only after a cryptographically validated code + PKCE flow (signature, issuer, audience, nonce, expiry, state) and before any session exists, so no session-authenticated server function can perform it. Recording the issuer is required by FR-009: without it, every new binding would be issuer-less and denied. | A server function needs a session, which does not exist yet. Omitting the issuer would make new bindings unusable. Widening the 3.0.0 exception to cover the callback was not done: the owner approved preserving this behavior, not a broader exception, so it is justified here for review under the existing governance rule. |

Analyze (finding C1) found this deviation. It was not listed when the 3.0.0
amendment was drafted. Only this callback write and the named `link-oidc` exception
are addressed; no other mutation path is assessed or waived.
