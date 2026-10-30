# Research: Operator-provisioned OIDC identity binding

Phase 0 for [plan.md](plan.md). The external sources listed in the spec had already
been reviewed before planning, so no new broad comparison was made. An independent
planning review covered persistence, legacy bindings, concurrency and test
infrastructure; its conclusions are folded into the decisions below. Everything
else comes from reading the code at base `c71a2756`.

Decision status: the **approved outcome** (command, authority, rejection rules,
generic OIDC, no UI or new providers) was fixed by the product owner. Decisions
D1–D8 are **technical decisions** made during planning and accepted in contract
review.

## Evidence

- Provider behavior: tsidp v0.0.14 `server/token.go` builds ID token claims without
  `email_verified`. A real code + PKCE exchange, with RS256 signature, issuer,
  audience, nonce, state and timestamps checked, showed the claim absent; the Horae
  callback was not reached and no session existed. No real subject, email, token or
  hostname is recorded here.
- `auth/oidc.rs`: `exchange_and_verify` validates the token with `openidconnect`
  (signature, issuer, audience, nonce, expiry) and maps a missing `email_verified`
  to `false`. `Identity` has no issuer. `fetch_by_subject` matches on
  `oidc_subject` only. The email path claims only an account with
  `oidc_subject IS NULL`, and only when the email is verified. The conditional
  `UPDATE ... WHERE oidc_subject IS NULL AND active` plus a reread by subject
  settles competing callbacks; `auth/oidc/tests/linking.rs` covers that.
- Schema: `users.oidc_subject text UNIQUE`, nullable, from `0001_init.sql`. No
  issuer column. The latest migration is `0048`.
- Sessions: the session stores only the account ID. `server_fns::require_user`
  reloads the account with `active = true` on every request, so deactivation
  revokes access whatever the binding.
- Shared DTO: `models::User` (serialized to the browser) carries `oidc_subject`.
  Every `query_as!(User, ...)` lists its columns explicitly, so a new nullable
  column does not have to join it.
- Existing logs: `callback` logs `OIDC token exchange/verification failed: {e}`,
  and an `openidconnect` validation error can name the issuer it rejected.
  That pre-dates this feature and is not changed by it.
- Plugin reads: `plugin/database.rs::validate_connection` rejects a plugin login that
  can `SELECT users.oidc_subject`. The documented grant
  (`contracts/plugin-interface.md`) is column-level:
  `id, org_id, email, name, org_role, active`. A new column is therefore not
  granted, but nothing checks that it stays that way.
- CLI: `user list` prints name, email, role and status, with no ID. `user create`
  and `init` write directly with the operator's database access.
- 014 personal settings: profile edits must not rebind the subject or treat an email
  edit as identity proof. Nothing here changes that contract.
- `nix/checks/e2e-oidc.nix`: Dex `mockCallback` with a verified email. Dex v2.45.1
  always serializes `email_verified` when the `email` scope is requested (the
  pointer is set in `newIDToken`), so stock Dex yields `false` at worst, never an
  absent claim.
- `openidconnect` 3.5: `IssuerUrl` keeps the original string; `as_str()` returns it
  as spelled, while `.url().as_str()` returns the parsed, normalized form.

## Decisions

### D1. Storage: a nullable issuer column on `users`

- **Decision**: add `users.oidc_issuer text NULL` with
  `CHECK (oidc_issuer IS NULL OR oidc_subject IS NOT NULL)`. Keep
  `UNIQUE (oidc_subject)` as it is.
- **Rationale**: one column and one constraint; no data migration; existing rows
  become legacy bindings (subject, no issuer) without being touched. A globally
  unique subject is stricter than (issuer, subject) uniqueness and, with one
  provider at a time, loses nothing. The same subject under a second issuer is
  therefore rejected in this increment.
- **Alternatives rejected**: a separate `user_oidc_identities` table (needs a data
  migration, an `org_id`, and changes every reader of `oidc_subject`, for a
  one-provider product); `UNIQUE (oidc_issuer, oidc_subject)` with
  `NULLS NOT DISTINCT` (lets two accounts hold the same subject under different
  issuers, which this feature has no use for); backfilling the configured issuer
  into old rows (the approved outcome forbids silent attribution).

### D2. Resolution: the issuer and subject pair only; legacy bindings are denied

- **Decision**: `Identity` gains `issuer` from the validated claims
  (`claims.issuer().as_str()`, see D6a). Lookup becomes
  `WHERE oidc_issuer = $issuer AND oidc_subject = $subject`. A legacy row (subject,
  no issuer) matches nothing, and because it is bound, the email path does not claim
  it either: sign-in is denied with no write. The email path's conditional UPDATE
  sets `oidc_issuer` and `oidc_subject` together. Sign-in never writes an issuer onto
  an existing binding.
- **Rationale**: the approved outcome resolves the pair and forbids silent issuer
  adoption, so completing a legacy binding is an explicit operator act. Keeping a
  subject-only match for legacy rows was considered and rejected for that reason.
- **Upgrade impact**: every account bound before the upgrade must be completed with
  `link-oidc` before its next OIDC sign-in. Open sessions continue (D8). The
  quickstart and README carry this as an upgrade step.
- **Concurrency of the email path**: the losing callback's reread uses the pair, so
  it succeeds only for the same issuer, subject and active account.
- **Subject already held elsewhere**: when a token's pair misses but its subject is
  held by another account (legacy, other issuer, or a concurrent `link-oidc`) and
  its verified email matches a different unbound account, the email path's UPDATE
  violates the unique subject index. The callback keeps its current behavior for
  that `Err`: logged, redirected to sign-in, no session, no binding change. No new
  error mapping is added there; only the CLI maps that 23505 to `SubjectInUse`.

### D3. The link decision is a pure function plus one locked transaction

- **Decision**: a new module `auth/oidc/link.rs` (beside the existing
  `auth/oidc/tests/` directory, under the `oidc.rs` module root) holds input
  validation, a pure `decide` function over the target row and the subject's current
  holder, and `link(pool, user_id, issuer, subject)`. `link` locks the target row
  (`SELECT ... FOR UPDATE`), reads the subject's current holder, decides and writes
  in one transaction. Different targets are not locked against each other; the
  `oidc_subject` unique index arbitrates between them. Only a unique violation
  (SQLSTATE 23505) whose constraint is `users_oidc_subject_key` maps to the
  `SubjectInUse` rejection; any other database error stays an error. The name is
  PostgreSQL's default for the `0001` column constraint and is confirmed against the
  migrated schema before the code relies on it (T001).
- **Linearization against deactivation**: deactivation updates the same row, so the
  row lock orders the two. If deactivation commits first, `link` sees
  `active = false` and rejects without writing. If `link` commits first, the binding
  stays (deactivation never clears bindings, as today), and the later deactivation
  still revokes access: sign-in denies inactive accounts and every request re-checks
  `active`. No interleaving overwrites a binding or puts one subject on two
  accounts.
- **Rationale**: the rules are the security contract, so they are unit-testable
  without a database (same pattern as `resolve_identity`). The row lock serializes
  against the callback's conditional UPDATE and against deactivation.
- **Not in `horae-core`**: Principle II names duration, money, totals and
  entry/invoice transitions. Identity binding is auth, which already keeps its pure
  decision in `auth/oidc.rs`.

### D4. Issuer check against configuration (accepted)

- **Decision**: if `HORAE_OIDC_ISSUER` is set and non-empty in the command's
  environment, the given issuer must equal it exactly. The other OIDC settings
  (client ID, secret, redirect URL) are not needed for this check. Otherwise the
  issuer is only validated. The check reads that one variable, not
  `AppConfig::oidc`, which exists only when all four settings are present.
- **Rationale**: a mismatched issuer can never sign in and cannot be replaced. The
  NixOS service keeps OIDC settings in the unit's environment, and the documented
  `sudo -u horae DATABASE_URL=... horae ...` invocation does not inherit them, so a
  hard requirement would break the documented flow.
- **Alternative rejected**: always require the configured issuer.

### D5. Exit codes (accepted)

- **Decision**: 0 for linked, unchanged and issuer recorded; 1 for any rejection or
  operational error (existing `anyhow` reporting), with the reason on stderr; 2 for
  usage errors (Clap).
- **Rationale**: matches the other local commands.
- **Alternative rejected**: a distinct code for rejections; no known caller needs it.

### D6. Full-flow proof without `email_verified` (accepted, pending test evidence)

- **Decision**: a test-only Dex derivation used by one isolated scenario and its own
  static client. A source patch omits the optional `email_verified` claim while Dex
  builds the ID token, before signing. Dex still performs authorization, the code +
  PKCE exchange and signing, and Horae validates the token as usual.
- **Seam**: Dex v2.45.1 `server/oauth2.go`, `newIDToken(ctx, clientID, …)`, contains
  exactly one `tok.EmailVerified = &claims.EmailVerified`. The test-only package is
  set through `services.dex.package` as an `overrideAttrs` of the nixpkgs Dex
  package (attribute name confirmed while implementing), whose `postPatch` uses
  `--replace-fail` to turn that line into
  `if clientID != "horae-omit-email-verified" { tok.EmailVerified = &claims.EmailVerified }`.
  `--replace-fail` fails the build if the seam moves.
- **Scenario layout** (one VM, the existing `e2e-oidc` check):
  1. The existing verified-email first-bind regression runs first, unchanged, with
     the normal `horae` client and no prior link.
  1. A repeat `link-oidc` for that user with Dex's issuer and subject reports
     "nothing changed", which proves sign-in stored the exact issuer string. It runs
     before the existing deactivation step, because `Inactive` is checked before
     `Unchanged`.
  1. The existing deactivation scenario.
  1. In a separate phase, reset the fixture user in the test database
     (`active = true`, `oidc_issuer` and `oidc_subject` both NULL), switch Horae's
     service to the dedicated `horae-omit-email-verified` client, and use fresh
     cookie jars.
  1. Sign-in is denied before any link.
  1. `link-oidc` links Dex's issuer and subject (exit 0), the repeat says nothing
     changed (exit 0), and a link of the same subject to another user fails (exit 1).
  1. Sign-in succeeds after the link.
- **Claim assertions**: the test gets an ID token from each client through the same
  signed code + PKCE flow, verifies the signature with an existing library (test-only
  Python `PyJWT` + `cryptography` in the VM, or the `openidconnect` verifier), and
  asserts `email_verified` absent for the omit client and `true` for the normal
  client. No handwritten RSA, modular exponentiation or PKCS padding.
- **Dependencies**: no new Rust or production dependency, and no `flake.lock` change.
  A test-only crypto package inside the VM check is allowed. Production auth scopes,
  configuration and packages are unchanged.
- **Labelling**: this is a tiny provider fixture patch. It is neither stock Dex nor
  acceptance against a real tsidp, and a Dex token with `email_verified: false` is
  never reported as absent-claim proof.
- **Before the heavy run**: confirm that `services.dex.package` (or a drop-in)
  actually applies, and show a dry run of the check for review.
- **Alternatives rejected**: a custom in-test provider (an Axum router signing tokens
  with `openidconnect`'s `CoreRsaPrivateSigningKey`), a Python provider, Hydra, or a
  live tsidp. Each adds a provider stack or needs access that was not granted.

### D6a. Exact issuer and subject values

- **Decision**: the issuer recorded at sign-in is the validated claim's
  `IssuerUrl::as_str()`, never `.url().as_str()`. The link command parses the issuer
  only to check its scheme, host, query and fragment, and stores the operator's
  original string. Neither value is trimmed, lowercased or otherwise rewritten;
  invalid values are rejected.

### D7. Privacy

- **Decision**: add `OR has_column_privilege('public.users', 'oidc_issuer', 'SELECT')`
  to the plugin validation, test that a role granted only that column is rejected,
  and state in the plugin contract that both identity columns are excluded. The
  issuer is never selected into `models::User`, page queries or plugin payloads. The
  link command's output and errors never echo the issuer or subject. `user list`
  gains the ID column only.
- **Bounded scope**: the subject's existing presence in `models::User` and any
  issuer text inside existing validation error logs are recorded above and left
  unchanged; changing them is a separate refactor this feature does not make.
- **Rationale**: the issuer identifies which provider an account uses; it is
  identity data like the subject.

### D8. Sessions

- **Decision**: no session change. Linking does not touch sessions. Existing sessions
  keep running until logout or expiry, and every request still checks that the
  account is active.
- **Rationale**: a link only binds an unbound account or completes its own legacy
  binding, so no session can end up belonging to a different person.
- **Limit**: the upgrade denies new OIDC sign-ins for legacy bindings, but a legacy
  user's session that is already open keeps working until logout or expiry. It is
  the same account, so no blanket sign-out is added. An operator who wants it ended
  deactivates the account.

## Agent context

The plan workflow's "update agent context" step has no script in this checkout
(`.specify/scripts/bash/` has only `check-prerequisites.sh`, `common.sh`,
`create-new-feature.sh`, `setup-plan.sh`, `setup-tasks.sh`). It was not run and no
substitute was written. AGENTS.md gains the new subcommand with the
implementation (task T019).
