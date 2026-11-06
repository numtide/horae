# Data Model: OIDC sign-in denial notice and opt-in auto-provisioning

No schema change.

## User account (`users`, unchanged)

A provisioned account is an ordinary row:

| Field | Value |
|-------|-------|
| `id` | new UUID v7 |
| `org_id` | the single organization (nothing is created unless exactly one exists) |
| `email` | the token's email, as sent |
| `name` | the `name` claim, trimmed, or the email's local part |
| `org_role` | `member` |
| `cost_rate_cents`, `billable_rate_cents` | NULL |
| `active` | true |
| `oidc_issuer`, `oidc_subject` | the token's `iss` and `sub`, as sent |

Constraints relied on: the existing UNIQUE on `email` and on `oidc_subject` (one account per
identity and per exact address) and `users_oidc_issuer_requires_subject`. The creating
statement adds a case-insensitive "no account holds this email" condition.

## Sign-in notice (session only)

Stored under one session key between `/auth/callback` and `/auth/login`, removed when read.

| Variant | Carries | Shown when |
|---------|---------|------------|
| `NoAccount` | the token's email, if any | nothing matches and provisioning is off, or the email or subject is held by another identity |
| `Deactivated` | — | the identity, or a verified email of an unbound account, resolves to an inactive account |
| `DomainNotAllowed` | email | provisioning is on and the domain is not listed |
| `EmailUnconfirmed` | email | provisioning is on, the domain is listed, and the claim does not count as verified |

## Provisioning policy (configuration)

| Setting | Type | Default |
|---------|------|---------|
| `HORAE_OIDC_AUTO_PROVISION_DOMAINS` | comma-separated domains, lowercased | empty = off |
| `HORAE_OIDC_TRUST_UNVERIFIED_EMAIL` | `1`/`true` | off |
