# Quickstart: validating Harvest people import and the Harvest section

## Prerequisites

Use a scratch PostgreSQL on a free port, never the developer database on 5432. Set the URL inside
the dev shell, because `nix develop` re-exports `DATABASE_URL`:

```sh
nix develop -c env DATABASE_URL=postgres://horae@127.0.0.1:<port>/horae cargo test -p horae --features server harvest
```

## US1: people import

1. `cargo test -p horae-core`: report serde accepts old reports and round-trips the people fields.
1. `cargo test -p horae --features server people`: matched, created, missing and duplicate email,
   ambiguous, idempotent re-run, dry run writes nothing, entries of created people, existing
   accounts untouched, durable resume, CSV creation from email rows.
1. `cargo test -p horae --features server scale`: the scale fixture, with users, still passes.
1. Against a deployment connected to the evidence Harvest account (never production), preview an
   import and run `horae jobs report <id>`. Expect a `summary.people` line and a `people` list
   ([contract](contracts/import-report.md)).

## US2: Harvest section

1. `cargo test -p horae --features server importers`: route parsing, navigation highlight, server
   function authorization, commit from preview, error summary.
1. `node crates/horae/tests/browser/harvest-section.cjs`: index, overview, runs, a run page and a
   reload. Grouped errors and missing people render, "Add all as deactivated" works, and "Commit
   this dry run" opens the new run. Checked at widths 360, 768 and 1440.
1. Expected: the index shows only Harvest sources, with no planned placeholders.

## Gates

Run `nix fmt`, `cargo clippy -p horae --features server`, the full
`cargo test -p horae --features server`, the touch + `cargo sqlx prepare --workspace -- --features server --all-targets`
steps and the offline check, and `cargo run -p horae-dioxus-lint`.
