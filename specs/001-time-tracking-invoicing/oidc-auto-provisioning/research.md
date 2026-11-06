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

- **Decision**: a single `INSERT ... ON CONFLICT DO NOTHING RETURNING`, then a re-read by
  issuer and subject.
- **Rationale**: the global unique constraints on `users.email` and `users.oidc_subject`
  already make a second row impossible; the loser re-reads the winner's committed row.
  Grafana handles its first-login race the same way (re-fetch on conflict). NetBird takes a
  global lock and re-checks.
- **Alternatives**: an advisory lock or locking the organization row (more moving parts,
  no extra guarantee).

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
  no-account notice. A deactivated account is named only when the person's own identity or
  verified email resolves to it.
- **Rationale**: NetBird names the account owner (masked); Horae names no administrator.

## R7. Account name

- **Decision**: the `name` claim (default locale), trimmed; otherwise the email's local part.
- **Rationale**: matches NetBird, Gitea and Grafana; the email is always present when
  provisioning, so a name always exists.
