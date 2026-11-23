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

Since step 4 the go-harvest client also runs the sync workflow: it pulls time entries, clients,
projects, tasks and users in pages of three, once by page number and once by following
`links.next`, and fails unless both walks return the same records, each once. It then edits one
time entry and checks that `updated_since` returns that entry and nothing else.

## 5. Pagination and sync (step 4)

1. `curl -H "Authorization: Bearer $TOKEN" -H "X-Forwarded-Proto: https" http://localhost:8080/v2/clients?per_page=1`
   answers with `per_page` 1 and links starting `https://localhost:8080/v2/clients`; `next`
   carries a `cursor`. `per_page=2001` is 422; without `per_page` a page holds up to 2000.

1. Following `links.next` gives the same clients as counting up `page`.

1. Rename a project, then `GET /v2/projects?updated_since=<a time before the rename>` returns that
   project only.

1. `GET /v2/expenses` returns an empty list with the full envelope.

1. Following `links.next` on `/v2/time_entries` while deleting an entry already read returns
   every other entry once; counting up `page` instead would skip one.

### Timing on 100,000 and 1,000,000 time entries

Measured on 2026-11-22 against the release build (`nix build .#default`), PostgreSQL from the dev
shell, an AMD Ryzen 9 9950X3D under WSL2. The fixture is the demo seed plus entries of one person
(20 a day, six project and task pairs), analysed after loading: first 100,000, then 900,000 more.
Times are `curl`'s total for `GET /v2/time_entries?per_page=2000` with the demo administrator's
personal access token, each the middle of three runs. Walks add up `curl`'s times only, and wait
out the token's limit of 100 requests per 15 seconds between runs.

| Request | 100,000 entries | 1,000,000 entries |
| --- | --- | --- |
| page 1 | 63 ms | 345 ms |
| middle page (25; 250) | 59 ms | 355 ms |
| last page (51; 501) | 46 ms | 345 ms |
| page 1 with `from` | 37 ms | |
| all pages by `page` | 2.95 s (51 pages, slowest 66 ms) | |
| all pages by `links.next` | 3.06 s (51 pages, slowest 70 ms), 100,010 distinct entries | |
| `updated_since` just before an edit of 3 entries | 9 ms | |

A page costs the same at any depth: one statement counts the list and picks the page's ids from
a list of ids and dates, and only those 2000 entries are read in full. Counting the whole list is
what grows with its size.

A manager reading the same person's entries through a management assignment, rather than an
administrator reading all, runs the per-entry management checks. Measured on that statement
alone with `EXPLAIN ANALYZE` in `psql`: 36 to 45 ms on 100,000 entries and 325 to 385 ms on
1,000,000, at page 1, the middle and the end. Both stay far under the 5 s statement limit that
reads run with.

## 6. Exactness

```sh
cargo test -p horae-core api_decimal
```

The property tests check that every minute value and every minor-unit value for currencies with
0, 2 and 3 decimals comes back unchanged (SC-005).
