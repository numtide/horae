# Quickstart: validate the sign-in notice and auto-provisioning

## Automated

```sh
# Unit tests (decision table, domain parsing, notice rendering)
cargo test -p horae --features server --lib auth::
# Integration tests on a scratch PostgreSQL (never the dev database on :5432)
DATABASE_URL=postgres://horae@127.0.0.1:<scratch-port>/horae cargo test -p horae --features server provisioning
# End to end against Dex: notices, provisioning with and without email_verified
nix build .#checks.x86_64-linux.e2e-oidc
```

## Manual, against any OIDC provider

1. Configure OIDC as in the README, without `HORAE_OIDC_AUTO_PROVISION_DOMAINS`.
1. Sign in as a person with no account. Expect the sign-in page to say
   "No Horae account exists for <your email>." Reload: the notice is gone.
1. Set `HORAE_OIDC_AUTO_PROVISION_DOMAINS=<your domain>` and restart. Sign in again.
   - Provider sends `email_verified: true`: you land in Horae; People lists you as an active
     member.
   - Provider omits the claim (tsidp): the notice says the address isn't confirmed. Set
     `HORAE_OIDC_TRUST_UNVERIFIED_EMAIL=1` only if people cannot choose their address at the
     provider, restart, and sign in: you land in Horae.
1. Deactivate that account in People and sign in again: the notice says the account is
   deactivated, and no second account appears.
1. Set the domain list to `*` and restart: the server refuses to start and names
   `HORAE_OIDC_AUTO_PROVISION_DOMAINS`.

See [contracts/sign-in.md](contracts/sign-in.md) for the full outcome table.
