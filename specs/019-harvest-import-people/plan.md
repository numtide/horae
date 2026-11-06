# Implementation Plan: Harvest people import and the Harvest import section

**Branch**: `feat/harvest-import-people` (PR A, US1), `feat/harvest-import-section` stacked on it
(PR B, US2) | **Date**: 2026-11-06 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/019-harvest-import-people/spec.md`

## Summary

US1 adds Harvest people to the import as a fourth catalog level, after clients, projects and tasks,
and before time entries. Each person resolves through provenance, then a unique normalized email,
and is otherwise created as a deactivated member with no rates and no sign-in identity. Time entries
find those people through the run cache, so a created person's history imports in the same run. The
report gains a `people` count, a bounded people list, and person-aware row errors. The CSV path
creates people only from rows with an email.

US2 replaces the single `/admin/importers` page with a Harvest section of addressable pages. It
adds a server-side error summary, commit-from-preview, and in-place "Add to Horae" for the people
who still do not match, and removes the placeholder planned sources. US2 follows the harvest-ux plan
(research.md R-9) with the user-approved answers.

## Technical Context

**Language/Version**: Rust (workspace toolchain from the Nix dev shell), Dioxus 0.7.9 fullstack

**Primary Dependencies**: sqlx (compile-time checked macros), axum, serde, ureq (Harvest HTTP),
existing `jobs` lease/checkpoint machinery

**Storage**: PostgreSQL. One additive migration: `ALTER TYPE harvest_entity_type ADD VALUE 'person'`

**Testing**: `cargo test -p horae-core`, `#[sqlx::test]` integration tests on a scratch PostgreSQL
(never :5432), the Harvest HTTP test server, and browser checks in `crates/horae/tests/browser/`
(PR B)

**Target Platform**: Linux server + WASM client

**Project Type**: Web application (single app crate plus pure `horae-core`)

**Performance Goals**: People add at most one savepoint per Harvest person (hundreds). The
15,000-error summary loads in one request without sending error rows to the browser.

**Constraints**: Dry runs persist nothing; durable checkpoints resume; existing reports and
checkpoints still load; no Harvest call outside fakes in tests.

**Scale/Scope**: Observed account has 39 unmatched people among ~13,700 entries; the scale test
fixture is the regression bound.

## Constitution Check

*GATE: passed before Phase 0 and re-checked after Phase 1.*

- **I. Exactness**: Pass. No rates are imported onto accounts. Entry money and minutes are unchanged.
- **II. Domain Purity**: Pass. `horae-core` gains only serde data: `EntityType::Person`,
  `ImportSummary.people`, `RowError.person/kind`, `ReportPerson`. There are no I/O dependencies.
- **III. Single Datastore**: Pass. The new data is PostgreSQL rows only, with no new service.
- **IV. Mutations Through Server Functions**: Pass with no amendment. Account creation is a domain
  write performed by the import job. An administrator authorizes the job through the authenticated
  `start_harvest_*` server functions, the same as the client, project, task and entry writes that
  004 and 005 justify. The US2 actions (`add_harvest_people`, `commit_harvest_import_preview`,
  `harvest_import_error_summary`) are `#[server]` functions that call `require_admin`. Neither the
  `link-oidc` exception nor the proposed sign-in exception (oidc-auto-provisioning, 4.0.0) is used
  or widened.
- **V. Reproducible Builds & Formatting Gate**: Pass. Run `nix fmt`, clippy, the `.sqlx` cache
  check and the Dioxus lint ratchet.
- **Data constraints**: Pass. UUID v7 for new accounts, every row has `org_id`, Postgres only.

Re-check after design: unchanged, and there are no violations, so Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/019-harvest-import-people/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── import-report.md     # report JSON additions (PR A)
│   └── harvest-section.md   # routes and server functions (PR B)
├── checklists/requirements.md
└── tasks.md
```

### Source Code

```text
crates/core/src/importers/harvest/types.rs      # EntityType::Person, summary.people, RowError fields, people list
crates/horae/migrations/0053_harvest_person_entity.sql
crates/horae/src/importers/harvest.rs           # job_report total, UserCreated dispatch after commit
crates/horae/src/importers/harvest/
├── api_source.rs        # ApiUser: optional email, is_active
├── parents.rs           # Parent::Person, duplicate-email pre-pass
├── resolve.rs           # resolve_person, create_person, CSV branch in resolve_user, error kinds
├── resolve/preview.rs   # ParentSnapshot.users
├── apply.rs             # CSV person outcome folding
├── streaming.rs         # dispatch created-user events after inline commit
├── streaming/durable.rs # parent_count includes users; dispatch after each parent batch commit
├── csv_source.rs        # (no change expected; it already carries email/name)
└── engine_tests/people.rs, sync_tests/people.rs   # new tests
crates/horae/src/pages/importers/presentation.rs   # "People" label (PR A)

# PR B
crates/horae/src/route.rs
crates/horae/src/pages/importers.rs (index) + pages/importers/harvest.rs + harvest/*.rs
crates/horae/src/server_fns/importers.rs (+ error summary, commit preview, add people)
crates/horae/src/jobs/report.rs (summarize)
crates/horae/src/server_fns/users.rs (insert_user gains `active`)
crates/horae/tests/browser/harvest-section.cjs
```

**Structure Decision**: Follow the existing importer layout. Engine tests get new submodule files
(`foo.rs` + `foo/`), not `mod.rs`.

## PR split

- **PR A (US1)**: core types, migration, engine, report, CSV branch, the "People" label in the
  existing page, the spec 019 docs, and the amendments to specs 004 and 005 and `harvest-parity.md`.
  The `.sqlx` cache is regenerated.
- **PR B (US2)**: everything under "PR B" above, the placeholder-source removal and browser checks.
  It is rebased on A once A merges.

## Complexity Tracking

None.
