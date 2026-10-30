# Feature Specification: Operator-provisioned OIDC identity binding

**Feature Branch**: `feat/oidc-identity-provisioning`

**Created**: 2026-10-30

**Status**: Draft (documents phase; product code waits for contract review)

**Parent**: `specs/001-time-tracking-invoicing`. Refines FR-001 (authentication) and
FR-002 (admin-created accounts; deactivated users cannot sign in) and makes SC-008
(an operator reaches a usable first administrator) reachable with identity providers
that never assert a verified email. It does not amend the parent spec. It relies on
the Principle IV local operator exception in constitution 3.0.0. It does not change
the 014 personal settings contract: profile edits still cannot bind or rebind an
identity.

**Input**: The existing `horae init` creates the organization and its first
administrator. A new local operator command,
`horae user link-oidc --user-id UUID --issuer URL --subject ID`, binds one provider
identity to one existing Horae account. It creates no session, changes no role and
creates no identity-provider user. The provider then signs that person in through
the normal OIDC flow. The feature is generic OIDC, not a special case for any one
provider.

## Context

Horae accounts are created by administrators (parent FR-002). On a first OIDC
sign-in, Horae binds the provider's subject to the account whose email matches, but
only when the signed ID token says the email is verified. Some providers, tsidp
v0.0.14 among them, never include `email_verified` in the ID token. With such a
provider nobody can complete a first sign-in, including the administrator created
by `horae init`, so the installation never reaches a usable state.

The binding today stores only the subject. OIDC Core makes the issuer and subject
together the stable identifier of a person; a subject alone is unique only within
one issuer. Horae has one configured provider at a time, but a binding that does
not record its issuer cannot tell one provider's subject from another's.

Inferring the binding from an unverified email would let anyone who controls an
address at the provider take the matching Horae account. The binding therefore
comes from the operator, who learns the person's issuer and subject from the
provider's own administration records or from a cryptographically validated ID
token, never from an email address.

Sources: [OIDC Core: claim stability](https://openid.net/specs/openid-connect-core-1_0.html#ClaimStability),
[tsidp v0.0.14 token claims](https://github.com/tailscale/tsidp/blob/v0.0.14/server/token.go),
and comparable pre-provisioning designs in
[Nextcloud user_oidc](https://github.com/nextcloud/user_oidc#pre-provisioning) and
[GitLab identity linking](https://docs.gitlab.com/development/authentication/#identity-linking-and-extern_uid).

## Clarifications

### Session 2026-10-30

The product owner's approval fixed the outcome: the command, operator authority,
generic OIDC, and the rejection rules. No question was put to the owner again. The
answers below are technical decisions taken in contract review within that outcome.

- Q: May an explicit link complete a legacy binding (same subject, no issuer)? → A:
  Yes, for the identical subject only, after documented operator verification; a
  different subject is never replaced and sign-in never adopts an issuer by itself.
- Q: Must linking or this upgrade sign anyone out? → A: No. Describe existing
  sessions as they are; no new blanket sign-out requirement.
- Q: Where may the recorded issuer be read? → A: Authentication only. It stays out of
  the shared user data and ordinary page queries, and is excluded from plugin reads
  like the subject, with an explicit plugin privilege check.
- Q: Does a legacy binding (subject, no issuer) still sign in after upgrade? → A: No.
  Sign-in needs the issuer and subject pair. Legacy
  bindings are denied until an explicit operator link records the issuer; Horae
  never adopts one silently.
- Q: May one subject be bound under two issuers? → A: No. A subject stays unique
  across all accounts in this single-provider increment, so cross-issuer reuse of a
  subject is rejected.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Bootstrap the first administrator with a provider that never verifies email (Priority: P1)

An operator installs Horae, runs `horae init`, and configures a provider whose ID
tokens carry no `email_verified` claim. They look up the administrator's issuer and
subject in the provider's records, run `horae user link-oidc` for the administrator's
account, and the administrator then signs in through the normal sign-in page.

**Why this priority**: Without it the installation never has a usable administrator
with such a provider (parent SC-008).

**Independent Test**: With a provider that issues a signed ID token without
`email_verified`, sign-in is denied before linking and succeeds after linking the
account to that token's issuer and subject.

**Acceptance Scenarios**:

1. **Given** an active account with no binding and a provider token without
   `email_verified`, **When** the person signs in before any link, **Then** sign-in
   is denied and no session is created, exactly as today.
1. **Given** that account, **When** the operator links it to issuer I and subject S,
   **Then** the command reports the binding as created and exits successfully,
   without creating a session or changing the account's role or any other field.
1. **Given** that binding, **When** the person signs in and the validated token
   carries issuer I and subject S, **Then** they receive a session for that account.
1. **Given** the same command is run again with the identical issuer and subject,
   **Then** it reports that nothing changed and exits successfully.

______________________________________________________________________

### User Story 2 - Linking can never move or steal an identity (Priority: P1)

An operator who mistypes a user ID or reuses a subject is stopped. Linking never
replaces a binding, never takes a binding from another account, and never binds a
deactivated account.

**Why this priority**: A wrong binding hands one person's account to another.

**Independent Test**: Each rejection case below exits with a failure, prints the
reason, and leaves every account's binding unchanged.

**Acceptance Scenarios**:

1. **Given** no account with the given ID, **When** the operator links it, **Then**
   the command fails with "user not found".
1. **Given** a deactivated account, **When** the operator links it, **Then** the
   command fails and the account stays unbound.
1. **Given** another account already holds subject S, **When** the operator links S
   to a different account, **Then** the command fails and neither account changes.
1. **Given** the account is bound to a different subject, or to the same subject
   with a different issuer, **When** the operator links it, **Then** the command
   fails and the existing binding is unchanged.
1. **Given** the account is deactivated or bound by a sign-in while the command
   runs, **Then** the operations take effect in one order: a deactivation ordered
   first makes the command reject; a sign-in binding ordered first is never
   overwritten; a binding ordered before a deactivation stays but grants no access.

______________________________________________________________________

### User Story 3 - Sign-in resolves the issuer and subject together (Priority: P1)

After this feature, Horae records the issuer with every new binding and resolves
sign-ins by issuer and subject together.

**Why this priority**: The binding the operator records must mean the same person
at sign-in, and it must not match another provider's identical subject.

**Independent Test**: An account bound to (I, S) signs in with a token for (I, S)
and is denied with a token for (J, S).

**Acceptance Scenarios**:

1. **Given** an account bound to (I, S), **When** a validated token carries (J, S)
   with J ≠ I, **Then** sign-in is denied and the binding is unchanged.
1. **Given** an active unbound account with email E, **When** a validated token with
   issuer I, subject S, email E and `email_verified` true arrives, **Then** the
   account is bound to (I, S) and receives a session, as today but with the issuer
   recorded.
1. **Given** a validated token whose `email_verified` is false or absent, **When** no
   binding matches, **Then** sign-in is denied even if the email matches an account.
1. **Given** a bound account is deactivated, **When** its identity signs in, **Then**
   sign-in is denied.

______________________________________________________________________

### User Story 4 - Existing bindings are completed explicitly (Priority: P2)

Accounts bound before this feature have a subject but no issuer. Horae cannot tell
which provider issued that subject, so after upgrade such an account cannot sign in
through OIDC until an operator who has verified the provider records the issuer
for that same subject.

**Why this priority**: Horae must not guess an old binding's issuer, and the
upgrade must leave operators a safe, documented way back for each existing user.

**Independent Test**: A legacy binding is denied after upgrade and still has no
issuer; an explicit link with the same subject records the issuer and sign-in then
succeeds; a link with a different subject fails.

**Acceptance Scenarios**:

1. **Given** an account with subject S and no issuer, **When** a validated token
   carries subject S, **Then** sign-in is denied, no session is created and the
   account still has no issuer recorded. Its existing sessions are not ended.
1. **Given** that account, **When** the operator links it to issuer I and subject S,
   **Then** the command reports the issuer as recorded and later sign-ins with
   (I, S) succeed.
1. **Given** that legacy account, **When** a validated token with a verified email
   matching it arrives, **Then** sign-in is denied: the account is bound, so the
   email path does not apply.
1. **Given** that account, **When** the operator links it to a different subject,
   **Then** the command fails and the binding is unchanged.

______________________________________________________________________

### User Story 5 - Find the account to link (Priority: P3)

The operator needs the target account's ID. `horae user list` shows each account's
ID next to its name, email, role and status.

**Independent Test**: `horae user list` output contains every account's ID.

**Acceptance Scenarios**:

1. **Given** accounts exist, **When** the operator runs `horae user list`, **Then**
   each line includes the account ID, and no line shows any issuer or subject.

### Edge Cases

- Empty subject, a subject longer than 255 bytes in UTF-8, or a subject with leading
  or trailing Unicode whitespace: rejected, never trimmed. Otherwise the subject is stored
  byte for byte, case preserved.
- Issuer that is not an absolute `https` URL (or `http` for a loopback host), or has
  a query or fragment: rejected. A valid issuer is stored exactly as given; a
  trailing slash is significant because token issuers are compared exactly.
- Issuer that differs from the issuer configured in the command's environment:
  rejected, because that binding could never sign in and cannot be replaced
  (FR-007). With no issuer configured there, the value is only validated.
- No organization yet: there are no accounts, so the command fails with "user not
  found"; `init` comes first.
- The person already holds a session in another browser: linking neither creates nor
  ends sessions (see FR-011).
- A provider change or a typo in a subject: there is no rebind, unlink or force
  option. Recovery is outside this feature (see Assumptions).
- Upgrade: every account bound before this feature stops signing in through OIDC
  until it is linked with its issuer. Its open sessions continue until logout or
  expiry. Operator documentation lists this as an upgrade step.
- The same subject under another issuer: rejected by the link command (the subject
  is held), and a token carrying it matches no account.
- A token whose subject is already held by another account and whose verified email
  matches a different unbound account: sign-in is denied with no session and no
  binding change. The existing error path handles it; nothing new is mapped.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide a local operator command
  `horae user link-oidc --user-id UUID --issuer URL --subject ID` that runs with the
  operator's database access and is not reachable over HTTP, from the UI or from
  plugins.
- **FR-002**: The command MUST bind (issuer, subject) to the target account only
  when the account exists and is active and either has no binding, or has the same
  subject with no issuer recorded.
- **FR-003**: When the account already has exactly this issuer and subject, the
  command MUST change nothing and succeed.
- **FR-004**: The command MUST fail without changing anything when the account does
  not exist, is deactivated, has a different subject, has the same subject with a
  different issuer, or when another account holds the subject.
- **FR-005**: The command MUST NOT create accounts, sessions or identity-provider
  users, and MUST NOT change roles, email, name, rates or active status.
- **FR-006**: The command's decision and write MUST be linearizable with concurrent
  sign-ins, deactivation and other link commands. If a deactivation commits first,
  the command binds nothing. If the binding commits first, it stays, and a later
  deactivation still revokes access (sign-in denies inactive accounts and every
  request re-checks the account). No interleaving may replace a binding or put one
  subject on two accounts.
- **FR-007**: The command MUST validate the subject (non-empty, at most 255 bytes in
  UTF-8, no leading or trailing whitespace as Unicode `White_Space` defines it, the
  set Rust's `str::trim` removes) and the issuer (absolute `https`
  URL, or `http` for a loopback host, with no query, fragment, control character
  or backslash, and exactly `//` before the host) without
  normalizing either value, and MUST store the subject byte for byte with its case.
  When `HORAE_OIDC_ISSUER` is set and non-empty in the command's environment, the
  given issuer MUST equal it exactly; no other OIDC setting is required for this
  check. A set value that is not valid UTF-8 MUST be refused, never skipped, and
  never printed. Validation and this check MUST complete before any database
  connection.
- **FR-008**: The command MUST print the outcome (linked, unchanged, issuer
  recorded, or the rejection reason) and exit 0 on success and non-zero on any
  rejection or error. It MUST NOT write the issuer or subject to logs.
- **FR-009**: Sign-in MUST resolve an account only by the validated token's issuer
  and subject together. An account bound to a different issuer, or whose binding
  has no issuer recorded, MUST NOT match, and sign-in MUST NOT record an issuer on
  an existing binding. Such a sign-in is denied without changing any account.
- **FR-010**: Automatic binding of an active, unbound account by email MUST remain,
  only when the validated token says the email is verified; a missing
  `email_verified` claim counts as unverified. The binding it creates MUST record
  the token's issuer. The email MUST NOT be used to resolve any other binding.
- **FR-011**: Linking MUST NOT create, end or alter sessions. Sessions belong to
  accounts, and every request still requires the account to be active (parent
  FR-002), so deactivation keeps revoking access and no blanket sign-out is added.
- **FR-012**: This feature MUST NOT add any new exposure of the issuer: it is not
  added to the shared user data sent to the browser, page data or plugin payloads,
  and the plugin database role MUST be refused if it can read either the issuer or
  the subject. The link command's output and error messages MUST NOT echo the issuer
  or the subject, and the command MUST NOT log them. Existing exposure is recorded
  in research.md and left unchanged: the shared user data already carries the
  subject, and an existing token-validation error log can name an issuer.
- **FR-013**: `horae user list` MUST show each account's ID and MUST NOT show issuer
  or subject.
- **FR-014**: Operator documentation MUST describe how to obtain the issuer and
  subject from provider administration records or a validated ID token, that an
  email address is never evidence of a subject, what to verify before completing a
  legacy binding, the upgrade step that completes every legacy binding, and that a
  wrong binding cannot be replaced by Horae.

### Key Entities

- **User account**: an existing Horae account (parent data model). It gains an
  optional recorded issuer alongside its optional subject. Subject without issuer is
  a legacy binding, which cannot sign in until completed; issuer without subject is
  not allowed.
- **OIDC identity**: the (issuer, subject) pair from a validated ID token or the
  provider's administration records. A subject is unique across all accounts.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: An operator can take a fresh installation with a provider that never
  asserts a verified email to a signed-in first administrator using only `init`,
  `user list`, `user link-oidc` and the normal sign-in page.
- **SC-002**: In every rejection case of User Story 2 and FR-007, 100% of attempts
  leave all bindings unchanged and exit non-zero.
- **SC-003**: Under concurrent link commands, sign-ins and deactivation, no run ends
  with a subject on two accounts or a replaced binding; a link ordered after a
  deactivation writes nothing; and a deactivated account is refused at sign-in and
  on its next request whichever committed first.
- **SC-004**: No sign-in with an unverified or absent email claim ever binds or
  resolves an account by email.
- **SC-005**: No binding existing before upgrade gains an issuer or signs in through
  OIDC without an explicit operator link, and each one signs in again after that
  link records its issuer.
- **SC-006**: After this feature, a plugin database role that can read the issuer or
  the subject is refused; the issuer is in no shared user data, page data or plugin
  payload; and neither value appears in `user list` or `link-oidc` output or in any
  log line the command writes.

## Assumptions

- One OIDC provider is configured at a time. Multiple simultaneous providers,
  provider changes and migrating bindings between providers are out of scope.
- Recovering from a wrong binding (typo, provider change) needs direct database
  administration outside Horae; this feature adds no rebind, unlink or force option.
- No new UI, invitations, provisioning of provider users, role mapping from claims,
  or permission model changes.
- Sessions continue to be cookie sessions keyed by account; this feature does not
  change their lifetime.
- The configured issuer is the value the provider's validated tokens carry in
  `iss`; the existing token validation already requires them to match.
- Existing `init` and `user create` behavior is unchanged.
