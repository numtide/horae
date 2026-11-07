# Feature Specification: OIDC sign-in denial notice and opt-in auto-provisioning

**Feature Branch**: `feat/oidc-auto-provisioning`

**Created**: 2026-11-06

**Status**: Draft

**Parent**: `specs/001-time-tracking-invoicing`. Amends FR-001 (a denied sign-in says why)
and FR-002 (accounts are admin-created, or created at first sign-in when the operator
enables it). Extends `oidc-identity-provisioning` FR-010 without changing it. Relies on
the Principle IV "OIDC sign-in provisioning" exception in constitution 4.0.0.

**Input**: A teammate signed in through a provider three times and landed back on the
sign-in page each time, with no explanation: no Horae account existed for them, and
sign-in denied silently. An administrator had to create the account by hand. The
operator wants (1) a clear reason on the sign-in page whenever a sign-in is refused
because of the account, and (2) an opt-in way to let people from their own email
domains create their account by signing in.

## Context

Horae accounts are created by administrators (parent FR-002). Sign-in resolves the
provider's issuer and subject to a bound account, or binds an active, unbound account
whose email the provider asserts as verified (`oidc-identity-provisioning` FR-009,
FR-010). Every other outcome is a silent redirect back to the sign-in page.

Some providers never send `email_verified` at all. tsidp v0.0.14, the Tailscale
identity provider, is one of them: the email it sends is the person's tailnet login,
which the person cannot choose. Other providers send `email_verified: false` for
addresses they have not confirmed. Horae treats the claim as three-valued: verified,
unverified and absent.

Comparable products were reviewed (NetBird, Grafana, Gitea, Immich, Outline,
Mattermost, Vaultwarden, Authentik; see `research.md`). Most create an active account
on first sign-in and trust the provider. Vaultwarden and Outline treat `email_verified`
as three-valued, and Vaultwarden has an operator switch for providers that omit it.
NetBird adds an optional approval queue; that is a planned follow-up here, not part of
this feature.

## Clarifications

### Session 2026-11-06

The owner's go-ahead for this increment fixed the answers below.

- Q: Are new accounts active immediately or held for approval? → A: Active. An approval
  queue (pending state, Approve/Reject for administrators) is a separate follow-up.
- Q: Which role and rates does a provisioned account get? → A: Member, with no cost or
  billable rate.
- Q: How are domains matched? → A: Exactly and case-insensitively against the part of
  the email after its last `@`. No wildcard and no subdomain matching in this increment.
- Q: What about providers that omit `email_verified`? → A: An operator switch,
  off by default, treats an absent claim as verified for provisioning only. An explicit
  `false` is always refused, and the switch never widens binding of an existing account
  by email.
- Q: May an account be re-created or re-activated by signing in? → A: No. If any account
  already holds the email, in any state, sign-in does not create one.
- Q: How does the reason reach the sign-in page? → A: As a one-time notice held in the
  person's own pre-sign-in session, never in the URL.
- Q: What does the notice reveal? → A: Only the person's own email and a reason that
  concerns only that email. It never says whether a different identity holds the
  address.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A refused sign-in says why (Priority: P1)

A person signs in through the provider but has no usable Horae account. Instead of
landing silently on the sign-in page, they see a notice that names their email and
tells them to ask an administrator.

**Why this priority**: This is the reported failure. It costs nothing to enable and
helps every deployment, whether or not auto-provisioning is on.

**Independent Test**: With auto-provisioning off, an unknown identity signs in and the
sign-in page shows the no-account notice with that identity's email; reloading the page
no longer shows it.

**Acceptance Scenarios**:

1. **Given** no account matches the identity and auto-provisioning is off, **When** the
   person completes sign-in at the provider, **Then** they return to the sign-in page
   with "No Horae account exists for <email>. Ask an administrator to add you." and no
   session is created.
1. **Given** the identity matches a deactivated account, by binding or by verified
   email, **When** the person signs in, **Then** the sign-in page says their account is
   deactivated and to ask an administrator, and no session is created.
1. **Given** the provider sent no email, **When** sign-in is refused for lack of an
   account, **Then** the notice says no account matches this sign-in, without an
   address.
1. **Given** a notice was shown, **When** the person reloads or revisits the sign-in
   page, **Then** the notice is gone.
1. **Given** sign-in fails for any other reason (provider error, state mismatch, token
   validation failure), **Then** the behaviour is unchanged: the sign-in page with no
   notice.

______________________________________________________________________

### User Story 2 - People from an allowed domain create their own account (Priority: P1)

The operator lists the email domains whose people may join. A person with a verified
email in one of those domains signs in for the first time and lands in Horae as an
active member, bound to their provider identity.

**Why this priority**: It removes the administrator round trip that started this work.

**Independent Test**: With `example.com` allowed, a first sign-in with a verified
`ada@Example.com` creates exactly one active member account bound to that issuer and
subject and receives a session; later sign-ins reuse it.

**Acceptance Scenarios**:

1. **Given** the domain is allowed and the email is verified, **When** an identity with
   no matching account signs in, **Then** one account is created with the member role,
   no rates, active, in the organization, bound to the token's issuer and subject, and
   the person receives a session.
1. **Given** that account, **When** the same identity signs in again, **Then** it signs
   in to the same account and nothing new is created.
1. **Given** the token carries a display name, **Then** the account is named after it;
   **given** it carries none or only whitespace, **Then** the account is named after
   the part of the email before the `@`.
1. **Given** an account is created, **Then** an account-created event is recorded with
   the sign-in method, the same event administrators' account creation emits.
1. **Given** the email's domain is not on the list, **When** the identity signs in,
   **Then** no account is created and the sign-in page says that the address's domain
   cannot create accounts and to ask an administrator.
1. **Given** the provider marks the email unverified, **When** the identity signs in,
   **Then** no account is created and the sign-in page says the provider did not
   confirm the address.
1. **Given** two first sign-ins for the same identity arrive at the same time, **Then**
   exactly one account exists afterwards and both sign-ins reach that account.

______________________________________________________________________

### User Story 3 - Providers that never assert a verified email (Priority: P2)

An operator runs a provider that omits `email_verified` and whose emails people cannot
choose, such as a tailnet login. They turn on the trust switch so that people from the
allowed domains can create their account by signing in.

**Why this priority**: It is the provider of the original report, but the switch is a
deliberate trust decision, so it is off by default.

**Independent Test**: With the domain allowed, a token without `email_verified` creates
an account only while the switch is on, and a token with `email_verified: false` never
does.

**Acceptance Scenarios**:

1. **Given** the switch is on and the claim is absent, **When** an identity from an
   allowed domain signs in, **Then** the account is created as in User Story 2.
1. **Given** the switch is off and the claim is absent, **Then** no account is created
   and the notice says the provider did not confirm the address.
1. **Given** the switch is on and the claim is `false`, **Then** no account is created.
1. **Given** the switch is on, an unbound active account holds the email, and the claim
   is absent, **When** the identity signs in, **Then** the account is not bound and no
   account is created: binding by email still requires a verified claim.

______________________________________________________________________

### User Story 4 - Signing in never resurrects or duplicates an account (Priority: P1)

An account that was deactivated stays deactivated, and an address already used by any
account is never used for a second one.

**Why this priority**: Deactivation is how administrators remove access (parent FR-002).

**Independent Test**: For each case below, an attempted sign-in with an allowed,
verified email leaves the accounts table unchanged.

**Acceptance Scenarios**:

1. **Given** a deactivated, unbound account holds the email, **When** an identity
   with that verified email signs in, **Then** no account is created or reactivated and
   the deactivated notice is shown.
1. **Given** a deactivated account bound to another identity holds the email, or the
   email is not verified, **When** the identity signs in, **Then** no account is created
   or reactivated and the no-account notice is shown, so the notice does not reveal
   another identity's account.
1. **Given** an active account holds the email but is bound to another identity,
   **When** a new identity with that email signs in, **Then** no account is created,
   nothing is rebound, and the no-account notice is shown.
1. **Given** the token's subject is already held by an account under another issuer,
   or by a binding with no issuer recorded, **When** it signs in, **Then** no account is
   created and the no-account notice is shown.

### Edge Cases

- Email letter case: an existing account whose email differs only in letter case holds
  the address for FR-008, and domains match case-insensitively.
- An email with several `@`: the domain is the part after the last one. An email with no
  `@`, or nothing after it, is not in any allowed domain.
- No organization exists yet (before `horae init`): nothing is created and the
  no-account notice is shown.
- The configured domain list is malformed (an empty item such as `a,,b`, `*`, an entry
  containing `@` or whitespace): the server refuses to start and names the variable.
- A display name longer than anyone needs is stored as given; the provider bounds it.
- The notice and email are shown as text; markup in a provider-sent email or name is
  never interpreted.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: When sign-in completes at the provider and the validated identity is
  refused because of the account, Horae MUST return the person to the sign-in page with
  a one-time notice giving the reason: no account, account deactivated, domain not
  allowed to create accounts, or email not confirmed by the provider.
- **FR-002**: The notice MUST name only the email from the person's own validated
  token, or no email when the token has none. It MUST NOT reveal whether another
  identity holds that address, and MUST NOT carry the email or reason in the URL.
  It MUST be shown once and then discarded.
- **FR-003**: Other sign-in failures (provider error, missing or mismatched state,
  token exchange or validation failure, internal error) MUST keep today's behaviour.
- **FR-004**: The operator MUST be able to enable auto-provisioning by listing email
  domains in `HORAE_OIDC_AUTO_PROVISION_DOMAINS` (comma-separated). Unset or empty
  MUST mean disabled. Entries are trimmed and matched case-insensitively and exactly
  against the part of the email after its last `@`. An empty item, `*`, or an entry
  containing `@` or whitespace MUST stop the server at startup with an error naming the
  variable.
- **FR-005**: When auto-provisioning is enabled, sign-in MUST create an account only when
  all hold: no account is bound to the token's issuer and subject; the token's subject is
  held by no account; no account holds the email (FR-008); the email's domain is
  listed; and the email counts as verified for provisioning (FR-006). It is created
  active, with the member role, no rates, in the single organization, and bound to the
  token's issuer and subject; then the person receives a session.
- **FR-006**: `email_verified` MUST be read as three-valued. For provisioning, `true`
  counts as verified; `false` never does; an absent claim counts only when the operator
  sets `HORAE_OIDC_TRUST_UNVERIFIED_EMAIL` (off by default). The switch MUST NOT change
  binding of an existing account by email (`oidc-identity-provisioning` FR-010), which
  still requires `true`.
- **FR-007**: Concurrent first sign-ins for one identity MUST produce at most one
  account, and each of them MUST either sign in to that account or be refused as FR-001
  describes. No interleaving may create two accounts for one identity or one email.
- **FR-008**: Sign-in MUST NOT create an account when any account, in any state and
  regardless of binding, holds the email, compared without regard to letter case. It
  MUST NOT reactivate an account, change any existing account, or create any role other
  than member.
- **FR-009**: The account's name MUST be the token's display name, trimmed, or, when it
  is missing or blank, the part of the email before its last `@`.
- **FR-010**: Each created account MUST emit the existing account-created event with
  sign-in as its method, and MUST be logged with the account's ID only, without email,
  issuer or subject.
- **FR-011**: Deployment documentation MUST describe both settings, that tsidp omits
  `email_verified`, and that the trust switch is meant only for providers where people
  cannot choose their email address (such as a tailnet login).

### Key Entities

- **User account**: unchanged (parent data model and `oidc-identity-provisioning`). A
  provisioned account is an ordinary active member account with a binding.
- **Sign-in notice**: a one-time reason (and optionally the person's own email) held in
  the pre-sign-in session between the callback and the sign-in page. Not stored
  anywhere else.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of sign-ins refused for no account, a deactivated account, a domain
  not allowed or an unconfirmed email show the matching notice on the sign-in page; none
  return silently.
- **SC-002**: With auto-provisioning on and an allowed, verified email, a new person
  reaches Horae signed in on their first attempt, with no administrator action.
- **SC-003**: In every refusal case of User Stories 2–4, the accounts table is unchanged
  afterwards.
- **SC-004**: Under concurrent first sign-ins for one identity, every run ends with
  exactly one account.
- **SC-005**: No sign-in with `email_verified: false` ever creates or binds an account,
  and no sign-in with an absent claim binds an account by email, whatever the switch.

## Assumptions

- One organization and one OIDC provider, as in the parent feature.
- Follow-up, not in this feature: an approval mode in which provisioned accounts wait
  for an administrator (pending state, Approve/Reject in People, `*` permitted only
  with approval), and an allowed-groups claim filter.
- No new UI beyond the notice on the sign-in page, no schema change, no change to
  administrator account creation, `horae init`, `user create` or `link-oidc`.
- Dev login (`DEV_LOGIN=1`) is unaffected.
