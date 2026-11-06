# Implementation Plan: OIDC sign-in denial notice and opt-in auto-provisioning

**Branch**: `feat/oidc-auto-provisioning` | **Date**: 2026-11-06 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/001-time-tracking-invoicing/oidc-auto-provisioning/spec.md`

## Summary

The OIDC callback stops redirecting silently when the account refuses a sign-in. It
stores a one-time notice in the pre-sign-in session, and `/auth/login` renders it.
When the operator lists email domains, an identity that matches no account and whose
email is eligible gets a new active member account, created by one conditional insert
that also enforces "no account holds this email" and survives concurrent first
sign-ins. No migration.

## Technical Context

**Language/Version**: Rust 2024 (workspace toolchain from the flake)

**Primary Dependencies**: Axum, `tower-sessions` (Postgres store), `openidconnect`, `sqlx`

**Storage**: PostgreSQL; `users` table unchanged (global UNIQUE on `email` and
`oidc_subject`, `oidc_issuer` from 0049)

**Testing**: unit tests in `auth/oidc.rs`, `auth/page.rs`, `config.rs`; `#[sqlx::test]` in
`auth/oidc/tests/`; NixOS `e2e-oidc` with Dex

**Target Platform**: Linux server (NixOS module, OCI image)

**Project Type**: web service (Dioxus fullstack; this feature touches only server-rendered
auth routes)

**Performance Goals**: no new work on the authenticated request path; sign-in adds at most
one insert and one lookup

**Constraints**: no schema change; no email, issuer or subject in logs; nothing in URLs

**Scale/Scope**: one organization, one provider

## Constitution Check

Constitution 4.0.0, checked before Phase 0 and again after Phase 1.

| Principle | Status |
|-----------|--------|
| I. Exactness | N/A. No durations or money. Provisioned accounts have no rates. |
| II. Domain purity | Pass. The resolution decision and the eligibility check are pure functions unit-tested without I/O, beside the existing `resolve_identity` in `auth`. Identity is outside Principle II's list, so `horae-core` is unchanged. |
| III. Single datastore | Pass. PostgreSQL only; new rows get a UUID v7 key and the organization's `org_id`. |
| IV. Mutations through server functions | Pass under the 4.0.0 "OIDC sign-in provisioning" exception. The callback binds only as before, and creates only an active member with no rates bound to the token's issuer and subject, for a listed domain, with a verified email or (operator opt-in) an absent claim; never with `email_verified: false`, never when any account holds the email, never reactivating or rebinding. Not reachable from the UI or plugins. The callback binding write that `oidc-identity-provisioning/plan.md` justified in Complexity Tracking is now covered by the same exception. |
| V. Reproducible builds | `nix fmt`, clippy, the full suite on PostgreSQL, the `.sqlx` cache check and `e2e-oidc` before merge. |

After design: unchanged. No Complexity Tracking entry is needed.

## Project Structure

### Documentation (this feature)

```text
specs/001-time-tracking-invoicing/oidc-auto-provisioning/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md  tasks.md
├── contracts/sign-in.md
└── checklists/requirements.md
```

### Source Code (repository root)

```text
crates/horae/src/
├── config.rs                       # AutoProvision + parsing; OidcConfig::from_env -> Result
├── auth/oidc.rs                    # tristate email_verified, eligibility, provisioning insert,
│                                   # notice flash, login_page reads it, UserCreated dispatch
├── auth/page.rs                    # LoginNotice + rendering above the call to action
├── auth/dev.rs                     # render(.., None)
├── auth/oidc/tests/linking.rs      # call-site updates (new signature)
├── auth/oidc/tests/link.rs         # call-site updates
├── auth/oidc/tests/provisioning.rs # new integration tests
└── plugin/event.rs                 # `method` doc widened to account creation
nix/checks/e2e-oidc.nix             # notice and provisioning phases
README.md, .env.example, nix/modules/nixos/horae.nix (doc only), AGENTS.md (Auth paragraph)
```

**Structure Decision**: everything stays in the existing `auth::oidc` module. The new test
file sits beside `linking.rs` and reuses its `identity` and `wait_for_links` helpers.

## Design

1. **Outcome type.** `resolve_user(db, identity, policy)` returns
   `Result<SignIn, LoginNotice>` inside `anyhow::Result`, where `SignIn { user, created }`.
   `resolve_and_login` stores the notice and dispatches `UserCreated` when `created`.
1. **Pure decision.** `resolve_identity` keeps its subject-first, verified-email-second
   order. Its final "no match" branch now consults `Eligibility` (Off, NoEmail,
   DomainNotAllowed, Unconfirmed, Allowed), computed by a pure function from the policy,
   the email and the tristate claim. Only `Allowed` yields `Resolution::Provision`.
1. **Insert.** `INSERT INTO users (...) SELECT ... FROM organizations WHERE (SELECT count(*) FROM organizations) = 1 AND NOT EXISTS (SELECT 1 FROM users WHERE lower(email) = lower($email)) ON CONFLICT DO NOTHING RETURNING ...`. Zero rows means: the email is held,
   the subject is held, there is no single organization, or a concurrent sign-in won. The
   callback then re-reads by issuer and subject: the winner's account signs in (or is
   refused if inactive); anything else is the no-account notice.
1. **Notice.** `LoginNotice` is a serde enum stored under one session key. `login_page`
   takes the `Session`, removes the key and passes the value to `page::render`, which
   HTML-escapes the email. Other failures never set it (FR-003).
1. **Config.** `HORAE_OIDC_AUTO_PROVISION_DOMAINS` and `HORAE_OIDC_TRUST_UNVERIFIED_EMAIL`
   join `OidcConfig` as `auto_provision: AutoProvision`. A malformed domain list fails
   `AppConfig::from_env`, so the server does not start.

## Analyze record

`/speckit-analyze` (read-only) ran after tasks: 11/11 requirements covered, no
placeholders, no duplications, no constitution conflicts. Findings: C1 (no task checked
that the notice is shown only once; T015 now checks it on reload), C2 (FR-003 relies on
the existing e2e failure paths), I1 (wording only). The author fixed C1 in tasks.md.

## Complexity Tracking

None.
