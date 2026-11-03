# Quickstart: Harvest OAuth client settings

## Prerequisites

A disposable PostgreSQL with `CREATEDB` and migrations applied, and `DATABASE_URL` pointing at it.

## Automated validation

```sh
cargo test -p horae-core client                                              # validation + disconnect rule
cargo test -p horae --features server --bin horae client_settings            # precedence, sealing, no key, change rules
cargo test -p horae --features server --bin horae job_endpoints_enforce_session_role_and_organization  # HTTP matrix
cargo test -p horae --features server --test import_jobs_ui                  # rendered OAuth app view
```

Expected: all pass. The HTTP matrix asserts that members and managers get `FORBIDDEN` on all three endpoints, that no response body contains the environment secret, and that saves are refused while the environment client is set.

## Manual check (page-managed client)

1. Start the server with `HORAE_HARVEST_ENC_KEY` set and without `HORAE_HARVEST_CLIENT_ID`, `HORAE_HARVEST_CLIENT_SECRET` and `HORAE_HARVEST_REDIRECT_URL`.
1. Sign in as an administrator, then open Import → Harvest → "Set up OAuth app". The redirect URI shows `<origin>/auth/harvest/callback` with Copy.
1. Enter a client ID and secret, then Save credentials. The pill reads Configured and the secret shows only as stored. Reloading never returns it.
1. Back on Harvest, Connect Harvest is available. The authorization URL carries the saved client ID and redirect URI. No restart is needed.
1. Remove the client: the importer reads "Not configured", and CSV import still works.

## Manual check (environment-managed and no key)

- With the environment pair set, the OAuth app panel shows "Configured by the environment", read-only.
- Without `HORAE_HARVEST_ENC_KEY`, the panel explains that the key is required and offers no save.
