# Authentication

People sign in to the web app through an OIDC provider. Local development uses a one-click
administrator login instead. Scripts and other applications use personal access tokens, described
in the [API guide](api.md). Sessions are stored in PostgreSQL; see
[Sessions and cookies](configuration.md#sessions-and-cookies).

## OIDC

OIDC is enabled only when all four required variables are set and `DEV_LOGIN` is off:

| Variable | Description |
|---|---|
| `HORAE_OIDC_ISSUER` | OIDC provider issuer URL (required) |
| `HORAE_OIDC_CLIENT_ID` | OIDC client ID (required) |
| `HORAE_OIDC_CLIENT_SECRET` | OIDC client secret (required) |
| `HORAE_OIDC_REDIRECT_URL` | Exact public callback URL registered with the provider, ending in `/auth/callback` (required) |
| `HORAE_OIDC_ADDITIONAL_AUDIENCES` | Extra `aud` values to trust (comma-separated); optional |
| `HORAE_OIDC_BUTTON_LABEL` | Sign-in button text; defaults to `Continue with SSO` |
| `HORAE_OIDC_AUTO_PROVISION_DOMAINS` | Email domains whose people may create their own account by signing in (comma-separated); optional, see [below](#creating-accounts-at-first-sign-in) |
| `HORAE_OIDC_TRUST_UNVERIFIED_EMAIL` | `1` treats a token without `email_verified` as verified when creating accounts; optional, see [below](#providers-without-email_verified) |

Unset or empty required values leave OIDC unconfigured; they do not enable the dev bypass. Use the
`HORAE_OIDC_*` names above, not the old unprefixed `OIDC_*` examples. `DEV_LOGIN=1` bypasses OIDC
even when all provider settings are present.

The client secret belongs in a secret file; on NixOS that is `services.horae.secretKeyFile` (see
[Secrets](deployment.md#secrets)). The provisioning settings are not secrets and belong in
`systemd.services.horae.environment`.

## Linking OIDC identities

On first sign-in, Horae links an account to the provider's issuer and subject when the ID token
says the account's email is verified. Some providers never send `email_verified`; with them, an
operator links each account instead:

```sh
sudo -u horae DATABASE_URL=postgres:///horae horae user list   # ID is the first column
sudo -u horae DATABASE_URL=postgres:///horae \
  HORAE_OIDC_ISSUER='<HORAE_OIDC_ISSUER from the service configuration>' \
  horae user link-oidc --user-id <ID> --issuer '<iss>' --subject '<sub>'
```

Take `--issuer` and `--subject` from the provider's own records for that person, or from the `iss`
and `sub` claims of an ID token whose signature, issuer, audience and expiry you have checked. An
email address is never evidence of a subject. The issuer must match the provider's `iss` exactly,
trailing slash included.

Copy `HORAE_OIDC_ISSUER` separately from the deployment's own configuration (the value the service
runs with), not from the same place as `--issuer`. The command then refuses a mismatched issuer
before touching the database. `sudo` does not pass the service's environment to the command, so
set this one variable explicitly; do not source the service's whole environment or secret file.
Without it the issuer is only checked for shape.

The command only binds an active account with no binding, or records the issuer on that account's
existing binding with the same subject. Running it again is safe. It never replaces a binding,
creates accounts or sessions, or changes permissions. Horae cannot undo a wrong link; that needs
direct database administration.

### Upgrading accounts linked without an issuer

Accounts linked before Horae recorded the issuer have a subject but no recorded issuer, and cannot
sign in through OIDC until linked. After confirming the configured provider is the one that issued
their subjects, run `link-oidc` for each with that issuer and the account's existing subject. Open
sessions keep working until logout or expiry.

## Creating accounts at first sign-in

By default only administrators create accounts. A sign-in that matches no account, or a
deactivated one, returns to the sign-in page with a notice that names the person's own email and
tells them to ask an administrator.

To let people from your own email domains create their account by signing in, list the domains:

```sh
HORAE_OIDC_AUTO_PROVISION_DOMAINS=example.com,example.org
```

A first sign-in then creates an active member account with no rates, bound to the provider's
issuer and subject, when all of these hold:

- the part of the email after its last `@` is one of the listed domains (case-insensitive, exact:
  `sub.example.com` is not `example.com`);
- the ID token says the email is verified (`email_verified: true`);
- no account already has that email, in any letter case and in any state. A deactivated account
  stays deactivated; signing in never re-creates or reactivates it.

Promote or deactivate the new account in People like any other. There is no wildcard: `*`, an
empty item or an entry with `@` stops the server at startup.

### Providers without `email_verified`

Some providers never send `email_verified`; tsidp (Tailscale) is one. Its email is the person's
tailnet login, which they cannot choose. For providers like that, set
`HORAE_OIDC_TRUST_UNVERIFIED_EMAIL=1` so that a missing claim counts as verified when creating an
account. A token that says `email_verified: false` is still refused, and the setting never lets a
sign-in claim an existing account by email: that still needs `email_verified: true`, or
`link-oidc`. Do not set it for a provider where people can pick their own address, or anyone there
could join under your domain.

## Development login

`DEV_LOGIN=1` puts a **Sign in as Admin** button on `/auth/login` that signs anyone in as an
administrator without a password. The local dev stack and the demo set it for you. It
bypasses OIDC, logs a warning at start, and the server refuses to start with it unless the bind
address is loopback. Never set it in production.

## Personal access tokens

`/api/v1` and the Harvest-compatible API authenticate with personal access tokens, created under
**My settings → API tokens**. A token acts with its owner's permissions. The [API guide](api.md)
explains how to create, use and revoke them, and [Harvest](harvest.md#harvest-compatible-api) how
Harvest clients send them. Apps use OAuth access tokens instead, on both APIs, limited to the
scopes the person granted ([Apps](api.md#apps)).
