# Tasks: Operator-provisioned OIDC identity binding

**Input**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/link-oidc-cli.md](contracts/link-oidc-cli.md),
[quickstart.md](quickstart.md)

**Tests**: Required. The task asks for TDD. Each new-behavior test is run first and
must fail for the intended reason (red) before the code that makes it pass. Tests
that pin existing behavior are marked *characterization* and must pass before and
after. No test is reported as passing unless it was run.

**Gates**: Nothing below starts until the contract review approves it. Any task that
builds, touches a database or runs a check needs a granted slot (one heavy job,
`max-jobs`/`build-jobs` 1, no incremental, 4 GiB watchdog). Commits are unsigned
and small; no push or PR without an explicit grant.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an open task)
- **[Story]**: User story from spec.md

## Phase 1: Setup

- [ ] T001 In a granted disposable migrated database, confirm the name of the unique constraint on `users.oidc_subject` (expected `users_oidc_subject_key`) and record it in specs/001-time-tracking-invoicing/oidc-identity-provisioning/research.md (D3)
- [ ] T002 Add crates/horae/migrations/0049_oidc_issuer.sql: `ALTER TABLE users ADD COLUMN oidc_issuer text` and constraint `users_oidc_issuer_requires_subject CHECK (oidc_issuer IS NULL OR oidc_subject IS NOT NULL)`; no backfill

## Phase 2: Foundational

- [ ] T003 [P] In crates/horae/src/auth/oidc/link.rs (new, declared from crates/horae/src/auth/oidc.rs), write unit tests for subject and issuer validation (red), then implement them: empty; leading/trailing ASCII space and a non-ASCII `White_Space` (U+00A0) rejected, inner space kept; 255 vs 256 ASCII bytes; 127 two-byte characters (254 bytes) accepted and 128 (256 bytes) rejected; case kept; `https`, `http` on `localhost`/`127.0.0.1`/`::1` accepted, `http` elsewhere, query, fragment, relative rejected; the original string is returned unchanged (trailing slash kept, never the parser's normalized form)
- [ ] T004 [P] In crates/horae/src/auth/oidc/link.rs, write unit tests for the pure `decide` function covering transition rows 1–8 of data-model.md (red), including the subject's holder being U itself (must give `Unchanged`/`IssuerRecorded`, never `SubjectInUse`), then implement it with "holder id ≠ U"
- [ ] T005 In crates/horae/src/auth/oidc.rs, add `issuer` to `Identity`, filled from `claims.issuer().as_str()`; update the `Identity` constructors in crates/horae/src/auth/oidc/tests/linking.rs; existing tests there are characterization and must still pass unchanged in meaning

## Phase 3: User Story 1 - Bootstrap without a verified email (P1) 🎯 MVP

**Goal**: an operator links an existing active account and that person can sign in.

**Independent test**: link creates the binding, a repeat is a no-op, and nothing
else about the account or its sessions changes.

- [ ] T006 [US1] In crates/horae/src/auth/oidc/tests/link.rs (new, declared from the `tests` module in crates/horae/src/auth/oidc.rs), write `#[sqlx::test]` tests (red): unbound active account → `Linked` with issuer and subject stored byte for byte; repeat → `Unchanged`; role, email, name, rates, active and the session table are unchanged
- [ ] T007 [US1] Implement `link(pool, user_id, issuer, subject)` in crates/horae/src/auth/oidc/link.rs: one transaction, `SELECT … FOR UPDATE` on the target, read the subject's holder, `decide`, write issuer and subject only; map only SQLSTATE 23505 on the constraint from T001 to `SubjectInUse`
- [ ] T008 [US1] Add `UserAction::LinkOidc { user_id: Uuid, issuer: String, subject: String }` in crates/horae/src/cli.rs with parse tests: all flags required, a non-UUID user ID rejected
- [ ] T009 [US1] Dispatch `link-oidc` in crates/horae/src/main.rs: run the T003 input validation before `db::create_pool` (today the pool opens before the `user` action is matched, so `link-oidc` is handled ahead of that); print the outcome text from contracts/link-oidc-cli.md, rejections on stderr with exit 1; never print or log issuer or subject; no migration run
- [ ] T010 [US1] In crates/horae/tests/oidc_link_cli.rs (new), run the real binary against a `#[sqlx::test]` database: `Linked` then `Unchanged` exit 0, one rejection exit 1, a usage error exit 2, and neither stream contains the issuer or subject

## Phase 4: User Story 2 - Linking never moves or steals an identity (P1)

**Goal**: every rejection writes nothing; races cannot produce a bad binding.

**Independent test**: each rejection exits non-zero and every account's binding is
unchanged.

- [ ] T011 [P] [US2] In crates/horae/src/auth/oidc/tests/link.rs, add tests: user not found, deactivated, other account holds the subject, bound to a different subject, bound to the same subject with another issuer, legacy with a different subject; each asserts the rejection and unchanged rows
- [ ] T012 [US2] In crates/horae/src/auth/oidc/tests/link.rs, add concurrency tests with a held row lock (pattern of `wait_for_links` in linking.rs): two links of one subject to two targets (exactly one wins, the other `SubjectInUse`); link versus an email-path callback for the same account (one binding, the loser gets no session unless it is the same pair); a callback whose verified email names unbound account M while `link-oidc` binds the same subject to U (no binding change on M, no successful resolution); FR-006 linearization against deactivation in both orders: deactivation committed first → `link` rejects `Inactive` and writes nothing; link committed first → binding stays and a later deactivation makes `resolve_user` deny (the per-request `require_user` re-check is characterization of existing behavior)
- [ ] T013 [US2] Issuer equality with a configured `HORAE_OIDC_ISSUER` (research D4): when `HORAE_OIDC_ISSUER` is set and non-empty, compare against it in crates/horae/src/main.rs before opening the pool, reading that variable directly (not `AppConfig::oidc`, which needs all four settings); test in crates/horae/tests/oidc_link_cli.rs for mismatch with only `HORAE_OIDC_ISSUER` set (exit 1, nothing written), match, and unset

## Phase 5: User Story 3 - Sign-in resolves the pair (P1)

**Goal**: only the issuer and subject pair resolves an account.

**Independent test**: (I, S) signs in; (J, S) is denied.

- [ ] T014 [US3] In crates/horae/src/auth/oidc/tests/linking.rs, add tests (red): bound (I, S) resolves with (I, S) and is denied with (J, S); the email path stores the token's issuer; `email_verified: false` is denied (characterization of the bool; an absent claim is only proven by the signed token in T021); the concurrent same-pair callbacks still both succeed and different subjects still yield one; subject S held by account L (legacy, and in a second case bound under another issuer) plus a token (I, S) whose verified email belongs to a different unbound active account M → `resolve_user` returns `Err` (existing behavior, no new mapping), the error's displayed text does not contain S, M stays unbound and L is unchanged
- [ ] T015 [US3] In crates/horae/src/auth/oidc.rs, replace `fetch_by_subject` with a pair lookup (`oidc_issuer = $1 AND oidc_subject = $2`) used by `resolve_user` and its reread, and set `oidc_issuer` in the email-path conditional UPDATE; `models::User` and its column lists stay as they are

## Phase 6: User Story 4 - Legacy bindings are completed explicitly (P2)

**Goal**: legacy bindings never sign in or gain an issuer silently.

**Independent test**: legacy denied with no write; after `link-oidc` with the same
subject, sign-in succeeds.

- [ ] T016 [US4] In crates/horae/src/auth/oidc/tests/linking.rs and crates/horae/src/auth/oidc/tests/link.rs, add tests: a legacy row is denied for its own subject and stays issuer-less; a matching verified email does not claim it; `link` returns `IssuerRecorded`, after which `resolve_user` with the pair succeeds

## Phase 7: User Story 5 - Find the account to link (P3)

- [ ] T017 [US5] Add the ID as the first column of `user list` in crates/horae/src/main.rs, and assert in crates/horae/tests/oidc_link_cli.rs that each account's ID appears and no issuer or subject does

## Phase 8: Polish and cross-cutting

- [ ] T018 [P] In crates/horae/src/plugin/database.rs, add `OR has_column_privilege('public.users', 'oidc_issuer', 'SELECT')` to the relation check and a test that a plugin role granted only `SELECT (oidc_issuer)` on `users` is rejected (red before the guard)
- [ ] T019 [P] Documentation: README.md (operator flow, evidence rule, upgrade step, no rebind); AGENTS.md CLI list; specs/001-time-tracking-invoicing/contracts/plugin-interface.md (issuer excluded like the subject); specs/001-time-tracking-invoicing/data-model.md (users field list); specs/015-scoped-permissions/contracts/permission-state.md (identity writers: pair binding and `link-oidc`)
- [ ] T020 In nix/checks/e2e-oidc.nix, after the existing verified-email sign-in with the normal `horae` client, run `horae user link-oidc` for that user with Dex's issuer and mock subject and assert exit 0 with "nothing changed", which proves sign-in stored the exact issuer string. This repeat runs before the existing deactivation step (`Inactive` is checked before `Unchanged`); keep the deactivation scenario after it
- [ ] T021 In nix/checks/e2e-oidc.nix, the absent-claim scenario from research D6: a test-only `services.dex.package` override whose `postPatch` uses `--replace-fail` on the one `tok.EmailVerified = &claims.EmailVerified` line in `server/oauth2.go` to skip it for client `horae-omit-email-verified`; a dedicated static client; after T020 and the deactivation step, reset the fixture user (`active = true`, `oidc_issuer` and `oidc_subject` both NULL), switch the Horae service to that client and use fresh cookie jars; assert sign-in denied, then `link-oidc` exit 0, repeat exit 0 "nothing changed", same subject to another user exit 1, then sign-in succeeds. Verify each client's ID token signature with an existing library (test-only Python `PyJWT` + `cryptography`, or the `openidconnect` verifier; no handwritten RSA) and assert `email_verified` absent for the omit client and `true` for the normal client. First confirm the override applies and present `nix build --dry-run .#checks.x86_64-linux.e2e-oidc` for review before the heavy run
- [ ] T022 Regenerate `.sqlx` in an own disposable migrated database (`touch` sources, `cargo sqlx prepare --workspace -- --features server --all-targets`), then the offline all-targets build, `cargo clippy -p horae --features server`, `nix fmt`
- [ ] T023 On the exact head, in a granted slot: `nix flake check` (or the checks the reviewer names), then request independent reviews

## Dependencies

- T001 → T007 (constraint name). T002 → every database test.
- T003, T004 → T007. T005 → T014, T015.
- US1 (T006–T010) first; US2 and US4 extend its test file and depend on T007.
- US3 (T014–T015) depends only on T002 and T005, so it can run beside US1.
- T017 depends on T010 (same test file). T020 depends on T009 and T015.
- T021 depends on T020 (same check, runs after it). T022 → T023 last.

## Parallel examples

- T003 with T004 (separate functions, both pure).
- After T007: T011 beside T014 (different test files).
- T018 and T019 at any time after T002.

## Implementation strategy

MVP is US1 plus US3, since a link is useless unless sign-in resolves the pair. Then
US2 (safety), US4 (upgrade), US5 (convenience) and polish. Commit after each green
task group.

## Traceability

| Requirement | Tasks |
|-------------|-------|
| FR-001 | T008, T009, T010 |
| FR-002, FR-003 | T004, T006, T007, T016 |
| FR-004 | T004, T011 |
| FR-005 | T006, T007 |
| FR-006 | T007, T012 |
| FR-007 | T003, T013 |
| FR-008 | T009, T010 |
| FR-009 | T014, T015, T016 |
| FR-010 | T014, T015 (bool); T021 (absent claim) |
| FR-011 | T006 |
| FR-012 | T009, T010, T015, T018 |
| FR-013 | T017 |
| FR-014 | T019 |
| SC-001 | T020, T021 |
| SC-002 | T011, T013 |
| SC-003 | T012 |
| SC-004 | T014 (unverified bool), T021 (absent claim) |
| SC-005 | T016, T019 |
| SC-006 | T010, T017, T018 |
