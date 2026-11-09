# Quickstart: validating Harvest API compatibility

Run inside `nix develop`, with the dev stack up (`process-compose up`) and `DEV_LOGIN=1`.

## 1. Integer ids and account id (step 1)

1. Sign in as Admin and open personal settings: the organization's account id is shown.
1. `curl -b <session> http://localhost:8080/harvest/v2/projects` returns integer `id`s. A project
   imported from Harvest has its original Harvest id.
1. `GET /harvest/v2/projects/<old uuid>` still works (D-1).

## 2. Personal access tokens (step 2)

1. Personal settings → API tokens → create "cli". Copy the token.
1. `curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/v2/users/me` returns the
   signed-in person; with `-H "Harvest-Account-Id: 999999"` it returns 403.
1. Revoke the token; the same request returns 401.
1. 101 requests inside 15 seconds: the last is 429 with `Retry-After`.

## 3. Time-entry writes (step 3, needs the approved amendment)

1. `POST /v2/time_entries` with `project_id`, `task_id`, `spent_date` and `hours=1.5` creates a
   90-minute entry, visible on the web timesheet with the same total (SC-003).
1. `POST` without `hours` starts a timer; a second `POST` stops the first one (D-6). The web app
   shows one running timer.
1. `PATCH .../stop` then `.../restart` resumes it.
1. `GET /v2/company` and `GET /v2/users/me/project_assignments` answer with the shapes a picker
   needs.

## 4. Compatibility suite

```sh
nix build .#checks.x86_64-linux.harvest-compat -L
```

Each client prints one line per workflow, pass or fail. The result table is copied into
[contracts/harvest-api-v2.md](contracts/harvest-api-v2.md) with each step (FR-072).

## 5. Exactness

```sh
cargo test -p horae-core api_decimal
```

The property tests check that every minute value and every minor-unit value for currencies with
0, 2 and 3 decimals comes back unchanged (SC-005).
