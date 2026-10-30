# Quickstart: Operator-provisioned OIDC identity binding

How an operator uses the feature, and how to validate it. Nothing below has been
run yet; the documents phase ran no build, database or provider.

## Operator flow

1. Bootstrap as today:
   `horae init --org-name … --admin-email … --admin-name …`.
1. Configure OIDC (`HORAE_OIDC_ISSUER`, client ID, secret, redirect URL) as in the
   README.
1. Find the account ID: `horae user list` (first column).
1. Get the person's issuer and subject from evidence, never from an email address:
   - the provider's own administration records for that person, or
   - a validated ID token: the `iss` and `sub` claims of a token whose signature,
     issuer, audience and expiry you have checked. A decoded token you have not
     validated is not evidence.
     The issuer must be exactly the provider's `iss`, including any trailing slash.
1. Link: `horae user link-oidc --user-id <ID> --issuer <iss> --subject <sub>`.
   Exit 0 means linked or already linked; anything else prints the reason and
   changed nothing.
1. The person signs in through the normal sign-in page.

Re-running the same link is safe. A wrong link cannot be replaced or removed
through Horae; recovering from one needs direct database administration.

## Upgrade step for existing installations

Accounts that signed in through OIDC before this release have a subject but no
recorded issuer. After the upgrade they cannot sign in through OIDC until each one
is linked:

1. Confirm that the configured provider is the one that issued these subjects (for
   example, the provider has not changed since they signed in, and its records show
   the same subject for each person).
1. For each such account, run `link-oidc` with the configured issuer and the
   account's existing subject. The command reports "Recorded the issuer…". A
   different subject is rejected.

Open sessions keep working until logout or expiry. Deactivate an account to end its
access immediately.

## Validation

Integration and unit tests (need a disposable PostgreSQL with `CREATEDB`):

```sh
DATABASE_URL=… cargo test -p horae --features server oidc
DATABASE_URL=… cargo test -p horae --features server plugin::database
cargo test -p horae --features server cli::tests
```

NixOS deployment check (heavy; only in a granted build slot):

```sh
nix build .#checks.x86_64-linux.e2e-oidc
```

The same check also runs the absent-`email_verified` full flow against a test-only
Dex derivation (research D6). That is a tiny provider fixture patch: it is neither
stock Dex nor acceptance against a real tsidp. A Dex token with
`email_verified: false` is not proof of the absent-claim case. Before the heavy
run, evaluate the check without building
(`nix build --dry-run .#checks.x86_64-linux.e2e-oidc`) and confirm that the
`services.dex.package` override reaches the unit.

## Requirement → proof

| Requirement | Proof | Tasks |
|-------------|-------|-------|
| FR-001, FR-005 | CLI parse test; link test asserts role, email, name, rates, active and sessions unchanged | T006, T008, T010 |
| FR-002, FR-003 | link outcomes Linked / Unchanged / IssuerRecorded | T004, T006, T016 |
| FR-004 | rejections: not found, inactive, other subject, other issuer, subject in use | T004, T011 |
| FR-006 | link vs link on two targets, link vs email callback, link vs deactivation | T012 |
| FR-007 | validation incl. trailing slash, case, Unicode whitespace, 255/256 UTF-8 bytes incl. multibyte, http loopback, query/fragment; issuer mismatch | T003, T013 |
| FR-008 | outcome text and exit codes from the real binary | T010 |
| FR-009 | pair matches, other issuer denied, legacy denied with no write | T014, T016 |
| FR-010 | email path records issuer; `false` denied (T014); absent claim in a signed token denied (T021 only) | T014, T021 |
| FR-011 | session table unchanged by link; `require_user` unchanged | T006 |
| FR-012 | plugin role with `oidc_issuer` grant rejected; issuer absent from `User`; nothing printed or logged | T010, T015, T018 |
| FR-013 | `user list` shows ID, no issuer or subject | T017 |
| FR-014 | README and this quickstart | T019 |
| SC-001 | Dex e2e: repeat link proves the stored issuer; absent-claim full flow with the test-only Dex fixture | T020, T021 |

The complete matrix is in [tasks.md](tasks.md#traceability).
