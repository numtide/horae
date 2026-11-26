# Quickstart: validating the mirror, parity and the jobs crate

Commands assume the dev shell and a database from `process-compose up postgres migrate`. Set
`DATABASE_URL` inside `nix develop -c env …` (the shell re-exports its own).

## 1. Jobs crate (PRs 1 to 3)

```sh
cargo test -p horae-jobs
DATABASE_URL=… cargo test -p horae-server jobs
DATABASE_URL=… cargo test -p horae --features server --test cli_imports
```

Expected: every existing job, importer and CLI test passes unchanged (US1). The crate's tests cover a
concurrency-key conflict returning the existing job, a stale heartbeat reclaimed after the stale
limit and a live one never reclaimed, the retry delays of a policy, a delayed job not claimed early,
priorities, and a daily schedule across both daylight-saving changes in `Europe/Madrid` (US9).

Build times for PR 1 (warm, after `touch crates/server/src/jobs.rs`, then `crates/jobs/src/lib.rs`):

```sh
time cargo build -p horae --features server
time cargo test -p horae-server --no-run
```

## 2. Mirror against the Harvest test double (PRs 4 to 6)

```sh
DATABASE_URL=… cargo test -p horae-server mirror
DATABASE_URL=… cargo test -p horae-api read_only
DATABASE_URL=… cargo test -p horae-harvest-api read_only
```

Expected: a pass applies changes of every entity in dependency order; a pass with no changes writes
nothing; an interrupted pass resumes from its watermarks; 429 waits for `Retry-After`; deleted
records are removed or archived by the reconciliation; every edit surface refuses a mirrored record
with `harvest_mirror_read_only` (contracts/mirror.md).

In the app: `HORAE_HARVEST_MIRROR=1 process-compose up`, connect Harvest (the test account only),
run a full import, open the Harvest section, turn the mirror on with a 1-minute interval, and watch a
pass appear on the status page. `horae mirror status --json` prints the same figures.

## 3. Parity (PRs 7 and 8)

```sh
cargo test -p horae-core parity
DATABASE_URL=… cargo test -p horae-server parity
```

Expected: every planted difference is found in its class and layer (FR-036), a first sighting is
pending and a second is unexplained, a rounding difference within the bound is `known/rounding`, and
an untouched import reports zero unexplained findings (SC-003).

`horae mirror parity --wait`, then `horae jobs findings <job id> --output parity.csv`.

## 4. Rehearsal on the Harvest test account (T071 to T074)

Before starting, record the authorization and the fixture plan in `specs/harvest-parity.md`. Create
disposable fixtures there (an entry with odd minutes, an invoice with rounding, a deleted entry, a
moved project), run a pass and a parity run, check each expected class, confirm the open points of
research R-2, R-3 and R-8, then delete the fixtures and record their ids and cleanup.

## 5. Cutover (PR 9)

On a test database: `horae mirror cutover --wait` after a clean parity run. Expected: the mirror is
off, no schedules remain, mirrored records are editable, and the audit log has the cutover with its
parity run. With unexplained findings the command exits with the conflict code until
`--override-unexplained COUNT` matches.
