# Research: OIDC sign-in denial notice and opt-in auto-provisioning

Sources were read, not run: NetBird (management server and dashboard, main @ 772d0b8,
2026-11-05), and Grafana, Gitea, Immich, Outline, Mattermost, Vaultwarden and Authentik
source trees.

## R1. Active accounts or an approval queue

- **Decision**: create active accounts in this increment; an approval queue is a follow-up.
- **Rationale**: most reviewed products create an active account on first sign-in and
  trust the provider. The operator controls eligibility through the domain list and the
  trust switch. An approval queue needs a schema change and administrator UI.
- **Alternatives**: NetBird's `UserApprovalRequired` (new users blocked and pending until
  approved, default on for new accounts; rejected users are deleted and may apply again).
  Deferred, with deactivation instead of deletion as the planned rejection.

## R2. Which identities may join

- **Decision**: an exact, case-insensitive list of email domains; no wildcard yet.
- **Rationale**: Horae holds rates, invoices and client data, so "anyone the provider
  authenticates" is too broad without approval. NetBird's self-hosted single-account mode
  admits every authenticated identity; that model returns with the approval follow-up as
  `*`, allowed only together with approval.
- **Alternatives**: a groups-claim filter (NetBird `JWTAllowGroups`, checked there after the
  user row exists). Deferred; when added it is checked before creating anything.

## R3. `email_verified`

- **Decision**: three-valued. Provisioning requires `true`; an absent claim counts only
  with `HORAE_OIDC_TRUST_UNVERIFIED_EMAIL`; `false` never counts. Binding an existing
  account by email is unchanged (requires `true`).
- **Rationale**: with no approval step, the email domain is the only admission check, and
  a provider that lets people pick unconfirmed addresses would otherwise admit anyone
  under an allowed domain. tsidp v0.0.14 omits the claim, and its email is the tailnet
  login, which the person cannot choose. Vaultwarden makes the same split with
  `SSO_ALLOW_UNKNOWN_EMAIL_VERIFICATION` (default off); Outline also distinguishes the
  absent claim. NetBird ignores the claim, relying on the provider and approval.
- **Alternatives**: trusting absent everywhere (would widen FR-010 binding, rejected);
  never trusting absent (tsidp deployments could not provision, rejected).

## R4. Concurrent first sign-ins

- **Decision**: an `INSERT ... ON CONFLICT DO NOTHING RETURNING` under a transaction-scoped
  advisory lock keyed on the lowercased email, then a re-read by issuer and subject.
- **Rationale**: the unique constraint on `users.oidc_subject` stops a second row for one
  identity, but the one on `users.email` is case-sensitive: `Ada@x` and `ada@x` from two
  identities would both pass a snapshot `NOT EXISTS (lower(email) = ...)` check. The lock
  orders creations per address, and a read-committed check after it sees the winner. The
  loser re-reads the winner's committed row. Every path that creates a user (sign-in,
  administrator `create_user`, CLI `user create`) takes the same lock through
  `db::lock_new_user_email`, after locking the organization row, so the paths cannot
  deadlock one another. `init` creates the first organization and its admin in one
  transaction, before any sign-in can provision, and `seed` is demo data; neither takes
  the lock. Grafana handles its
  first-login race with a re-fetch on conflict; NetBird takes a global lock and re-checks.
- **Alternatives**: a unique index on `lower(email)` (a migration, and existing data may
  already hold case variants); locking the organization row (serializes every sign-up).

## R5. How the reason reaches the sign-in page

- **Decision**: a one-time notice in the server-side session; `/auth/login` removes and
  renders it.
- **Rationale**: the session already exists before sign-in (it carries the OIDC flow
  secrets). Keeping the email out of the URL keeps it out of access logs, history and
  `Referer`, and nobody can craft a link that shows arbitrary text on the sign-in page.
- **Alternatives**: Outline's `?notice=<code>` query codes (fine for fixed text, but the
  notice here names the email); NetBird's dashboard error route (Horae's sign-in page is
  outside the SPA, so a separate route adds nothing).

## R6. What the notice may reveal

- **Decision**: only the person's own token email and reasons about that email. An email
  held by a different identity, or a subject held under another issuer, gets the generic
  no-account notice without the address, since "no account exists for <email>" would be
  false there. A deactivated account is named only when the person's own identity or
  verified email resolves to it.
- **Accepted trade-off**: with provisioning off, a sign-in whose email is held by an
  account bound to another identity, or whose unverified email matches an account, still
  reads "No Horae account exists for <email>". Telling those cases apart would need an
  extra lookup, and the difference in wording would let someone who can choose an
  unverified address at the provider probe which addresses hold Horae accounts. Not
  revealing held versus free addresses outweighs the imprecise wording.
- **Rationale**: NetBird names the account owner (masked); Horae names no administrator.

## R7. Account name

- **Decision**: the `name` claim (default locale), trimmed; otherwise the email's local part.
- **Rationale**: matches NetBird, Gitea and Grafana; the email is always present when
  provisioning, so a name always exists.
