# Contract: OIDC sign-in outcomes and settings

## Settings

- `HORAE_OIDC_AUTO_PROVISION_DOMAINS`: comma-separated email domains, e.g.
  `example.com,example.org`. Unset or empty: off. Each item is trimmed and lowercased.
  Startup fails, naming the variable, for an empty item, `*`, or an item containing `@`
  or whitespace.
- `HORAE_OIDC_TRUST_UNVERIFIED_EMAIL`: `1` or `true` (any case) turns it on. Only affects
  provisioning, and only when the token has no `email_verified` claim.

## `GET /auth/callback` outcomes

After the existing state, PKCE and ID-token validation succeed:

| Case | Result | Notice on `/auth/login` |
|------|--------|-------------------------|
| issuer + subject bound to an active account | session, redirect `/` | — |
| issuer + subject bound to an inactive account | redirect `/auth/login` | Deactivated |
| `email_verified: true`, exact email on an active unbound account | bind, session, `/` | — |
| `email_verified: true`, exact email on an inactive unbound account | redirect | Deactivated |
| provisioning off, nothing else matched | redirect | NoAccount |
| no email in token, nothing matched | redirect | NoAccount (no address) |
| domain not listed | redirect | DomainNotAllowed |
| domain listed, claim `false`, or absent without the trust switch | redirect | EmailUnconfirmed |
| domain listed, claim counts as verified, insert created the account | session, `/`, `UserCreated` | — |
| insert created nothing; a concurrent sign-in created this identity's account | as for a bound account | — |
| insert created nothing otherwise (email or subject held, no single organization) | redirect | NoAccount (no address) |
| verified-email binding lost to a concurrent sign-in | redirect | NoAccount (no address) |

Every other failure (provider `error`, missing flow, state mismatch, exchange or
validation failure, internal error) redirects to `/auth/login` with no notice, as before.

## Notice text

| Notice | Text |
|--------|------|
| NoAccount (email) | No Horae account exists for {email}. Ask an administrator to add you. |
| NoAccount (none) | No Horae account matches this sign-in. Ask an administrator to add you. |
| Deactivated | Your Horae account is deactivated. Ask an administrator to restore access. |
| DomainNotAllowed | No Horae account exists for {email}, and addresses at {domain} can't create one. Ask an administrator to add you. |
| EmailUnconfirmed | No Horae account exists for {email}, and your identity provider hasn't confirmed this address, so Horae can't create one. Ask an administrator to add you. |

The notice is shown once; `{email}` and `{domain}` are HTML-escaped.

## Event

`user_created` with `user.method = "oidc"`, emitted once per created account.
