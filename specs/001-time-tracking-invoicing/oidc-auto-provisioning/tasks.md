# Tasks: OIDC sign-in denial notice and opt-in auto-provisioning

**Input**: design documents in `specs/001-time-tracking-invoicing/oidc-auto-provisioning/`

**Tests**: requested by the spec and the owner (integration, unit and e2e).

## Phase 1: Setup

- [ ] T001 Amend constitution to 4.0.0 (Principle IV second exception) in `.specify/memory/constitution.md`
- [ ] T002 Amend parent FR-001/FR-002 in `specs/001-time-tracking-invoicing/spec.md` and add the FR-010 cross-reference in `specs/001-time-tracking-invoicing/oidc-identity-provisioning/spec.md`

## Phase 2: Foundational

- [ ] T003 Add `AutoProvision { domains, trust_unverified_email }`, its env parsing (FR-004 errors) and unit tests; make `OidcConfig::from_env` fallible in `crates/horae/src/config.rs`
- [ ] T004 Read `email_verified` as `Option<bool>` in `Identity`, keeping binding on `Some(true)` only, in `crates/horae/src/auth/oidc.rs`
- [ ] T005 Change `resolve_user` to take the policy and return `Result<SignIn, LoginNotice>`; update callers in `crates/horae/src/auth/oidc/tests/linking.rs` and `crates/horae/src/auth/oidc/tests/link.rs`

## Phase 3: User Story 1 — a refused sign-in says why (P1)

**Independent test**: an unknown identity with provisioning off sees the no-account notice once.

- [ ] T006 [US1] Add `LoginNotice`, its text, and rendering above the call to action, with escaping tests, in `crates/horae/src/auth/page.rs`; pass `None` from `crates/horae/src/auth/dev.rs`
- [ ] T007 [US1] Map denials to notices, store the notice in the session in `resolve_and_login`, and read-and-remove it in `login_page` in `crates/horae/src/auth/oidc.rs`
- [ ] T008 [US1] Unit tests for the decision table's denial branches in `crates/horae/src/auth/oidc.rs`

## Phase 4: User Story 2 — allowed domains create their own account (P1)

**Independent test**: allowed domain + verified email creates one active member bound to issuer and subject.

- [ ] T009 [US2] Pure `eligibility(policy, email, email_verified)` and `Resolution::Provision` with unit tests in `crates/horae/src/auth/oidc.rs`
- [ ] T010 [US2] Conditional `INSERT ... ON CONFLICT DO NOTHING RETURNING` with the re-read on conflict, the name fallback, and `UserCreated` + `tracing::info!` in `crates/horae/src/auth/oidc.rs`; widen the `method` doc in `crates/horae/src/plugin/event.rs`
- [ ] T011 [US2] Integration tests: provisioned, disabled, domain mismatch, unverified, name fallback, second sign-in reuses, and the concurrent race in `crates/horae/src/auth/oidc/tests/provisioning.rs`

## Phase 5: User Story 3 — providers without `email_verified` (P2)

- [ ] T012 [US3] Integration tests: absent + switch → provisioned; absent without switch → denied; `false` + switch → denied; switch never binds by email, in `crates/horae/src/auth/oidc/tests/provisioning.rs`

## Phase 6: User Story 4 — never resurrect or duplicate (P1)

- [ ] T013 [US4] Integration tests: deactivated unbound (deactivated notice), deactivated bound elsewhere, active bound elsewhere, case-variant email, subject held under another issuer or legacy binding, in `crates/horae/src/auth/oidc/tests/provisioning.rs`

## Phase 7: Polish

- [ ] T014 [P] Document the settings and tsidp in `README.md`, `.env.example`, `nix/modules/nixos/horae.nix` (description only) and the Auth paragraph of `AGENTS.md`; update the module docs in `crates/horae/src/auth/oidc.rs` and `crates/horae/src/auth/page.rs`
- [ ] T015 Extend `nix/checks/e2e-oidc.nix`: notices for no account, domain not allowed, unconfirmed and deactivated, each gone on reload; provisioning with the verified client and with the omitting client plus the trust switch
- [ ] T016 Regenerate `.sqlx/` (touch + prepare) and verify with the offline build
- [ ] T017 Gates: `nix fmt`, clippy, full `cargo test -p horae --features server` on a scratch PostgreSQL, `nix build .#checks.x86_64-linux.e2e-oidc`, browser screenshot of the notice

## Dependencies

T001–T002 → T003–T005 → US1 (T006–T008) → US2 (T009–T011) → US3 (T012), US4 (T013) → T014–T017.
US3 and US4 tests can be written in parallel once T010 lands. T014 is independent of code.

## Implementation strategy

US1 alone fixes the reported loop. US2 adds provisioning, US3 the tsidp path, US4 locks down
re-creation. One PR, committed as: constitution; specs; implementation and tests.
