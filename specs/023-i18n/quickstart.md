# Quickstart: checking the first increment

How to verify each PR of the first increment and the increment as a whole. Commands run from the
repository root inside `nix develop`. Contracts: [catalog.md](contracts/catalog.md) and
[lint-rule.md](contracts/lint-rule.md).

## Prerequisites

- `nix develop`; a PostgreSQL from `process-compose up postgres migrate` for the `horae` tests.
- For the WASM measurement only: a release build, `cd crates/horae && dx build --release --locked`
  (several minutes; run it under the worktrees' build lock if other builds share the machine).

## After I-1 (infrastructure)

```sh
cargo test -p horae-i18n                     # generator, fallback, plurals, split_tag, unused messages
cargo clippy -p horae-i18n --all-targets -- -D warnings
cargo clippy -p horae-i18n --target wasm32-unknown-unknown -- -D warnings
cargo test -p horae-dioxus-lint              # fixtures, rule names its skill heading and evidence
cargo run -p horae-dioxus-lint               # every rule at its baseline; prints the info line
nix build && nix build .#checks.x86_64-linux.tests   # crane accepts the crate's build.rs (R-9)
```

Expected: all pass, and the lint prints `dioxus/literal-text: N (baseline N)` with N the count
of that day (about 1,420 on `af6521cb`).

Negative checks, each undone afterwards:

1. Add `p { "Hello" }` to any screen. `cargo run -p horae-dioxus-lint` fails, naming the file and
   line (User Story 1, scenario 3).
1. Call `tr.tasks_title_typo()` from a screen. `cargo build -p horae --features server` fails, and
   so does the WASM build (scenario 2).
1. Add `unused-probe = Probe` to `horae.ftl`. `cargo test -p horae-i18n` fails listing
   `unused-probe` (scenario 4).
1. Add `locales/xx/horae.ftl` containing `nonexistent = X`. The build fails; with only a subset of
   English's messages instead, it warns with the count (FR-005).

## Per-PR check (I-2 to I-12)

1. **Before migrating.** List the files open design branches change and compare with the area's:

   ```sh
   git fetch --no-prune https://github.com/numtide/horae.git \
     master:refs/remotes/origin/master '+refs/heads/design/*:refs/remotes/origin/design/*'
   for b in $(git branch -r --list 'origin/design/*'); do echo "$b"; git diff --name-only "origin/master...$b"; done
   ```

   Any overlap is agreed with that branch's author before starting (research R-10).

1. **First commit: the dump test.** Add the HTML dump test to each test file that renders the
   area's screens (research R-7), with no source change. Take the "before" dump:

   ```sh
   mkdir -p ../i18n-dumps/before
   HORAE_HTML_DUMP=$PWD/../i18n-dumps/before cargo test -p horae --features server --test <file> html_dump
   ```

   (For a screen tested inside the crate, `--bin horae html_dump` instead of `--test <file>`.)

1. **Migrate**, then take the "after" dump the same way into `../i18n-dumps/after`, and compare
   with the random parts removed:

   ```sh
   norm() { sed -E 's/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/UUID/g; s/[0-9]{2}:[0-9]{2}:[0-9]{2}/TIME/g' "$1"; }
   for f in ../i18n-dumps/before/*.html; do diff <(norm "$f") <(norm "../i18n-dumps/after/$(basename "$f")") || echo "DIFF $f"; done
   ```

   Expected: no output (SC-002). The dumps live outside the worktree and are not committed.

1. **Existing tests unchanged.** `git diff origin/master -- crates/horae/tests` shows only the
   dump test (I-1 already added the `mod i18n` includes, research R-8); no assertion changes. Then:

   ```sh
   cargo test -p horae --features server <the area's tests>
   cargo test -p horae-i18n
   cargo run -p horae-dioxus-lint
   ```

   The lint passes with the area's files lowered in `baseline.toml` (at 0, removed).

1. **PR description** records: the files migrated; the lint's `dioxus/literal-text` total and the
   info count of literals outside markup, before and after; the dump comparison result and which
   states it covered; the `NOT_LANGUAGE` entries added, with reasons; the overlap check of step 1.

1. **Merge check**: `nix flake check` green.

## WASM budget (I-1, I-2, I-13)

On the PR's base and on its head:

```sh
(cd crates/horae && dx build --release --locked)
stat -c %s target/dx/horae/release/web/public/assets/horae_bg-*.wasm.br
```

Clear `target/dx/horae/release/web/public/assets/` between the two builds so only one `.wasm.br`
remains. At I-13 the base is I-1's base commit (master before the increment), not the PR's own
base, so the number covers the whole increment. The difference is at most 60 KB (61,440 B) at
I-13 (SC-005); I-1 is expected
near 42 KB. Past 50 KB before I-13, stop and ask (research R-9).

## The increment as a whole (after I-13)

| Check | How | Expected |
| --- | --- | --- |
| SC-001 | `cargo run -p horae-dioxus-lint` with no `["dioxus/literal-text"]` table in `baseline.toml` | passes; the gallery and `NOT_LANGUAGE` are the only exceptions |
| SC-002 | every area PR's description | an empty dump diff per PR |
| SC-003 | PR-10's diff: `git diff --name-only <base>...<pr-10>` | only `crates/i18n/locales/en/horae.ftl` and test files |
| SC-005 | WASM budget above | ≤ 61,440 B brotli over master |
| FR-006 | `cargo test -p horae-i18n` | no unused message |
| FR-003 | `cargo tree -p horae-core -i horae-i18n` and the same for `horae-types` | both report that `horae-i18n` is not in the tree |
| FR-012 | `git diff --name-only <I-1 base>...<I-13>` | no file under `crates/server/src/ops`, `crates/api/src/auth`, `crates/api/src/exports*`, `crates/server/templates`, `crates/horae/src/cli` |
| FR-023 | the lint's info line | quoted in each PR; near zero after I-12, the rest false positives |
| Visual spot check | `process-compose up`, sign in as Admin, open Settings, Tasks, Projects and the Timesheet | the same words as on master |

## Not checked in the first increment

The sign-in page, `<html lang>`, language preferences, server messages, the invoice PDF and the
budget email (FR-012); they have their own increments and checks (SC-004, SC-006, SC-008).
