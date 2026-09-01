# Contract: Importer Interface (server function + CLI)

The importer is exposed through the existing authorized server-side path (Constitution IV) — an admin-only Dioxus `#[server]` function and/or a server-binary CLI subcommand. Both call the **same engine**, so behavior is identical regardless of surface. Signatures are intent-level (types abbreviated); server errors use the repo's `ServerFnError` with named status codes (e.g. `FORBIDDEN`), not integer literals.

## Server function (admin-only, backs the upload screen)

```
import_harvest_csv(
    file: Vec<u8>,          // uploaded CSV bytes
    mode: ImportMode,       // DryRun | Commit  — a named enum, NOT Option<bool>
) -> Result<ImportReport, ServerFnError>
```

- **Authorization**: rejects non-administrators with `FORBIDDEN` (FR-001). Reads the acting admin + single org from the session/`AppState`.
- **Validation**: rejects an unrecognized/empty file up front with a clear message and no writes (FR-003).
- **`mode = DryRun`**: runs the full parse → resolve → plan pipeline against live data and returns the report with **zero writes** (FR-014). `mode = Commit`: applies the plan (FR-015).
- **`ImportMode`** is a plainly named two-state enum per the repo's "avoid `Option<bool>`" rule.

## CLI subcommand (operator, large-file / host-side migration)

```
horae import harvest <FILE> [--dry-run]
```

- Runs on the `server` binary, sharing the same engine and DB layer as the server function.
- `--dry-run` selects `ImportMode::DryRun`; default is `Commit`.
- Prints the summary table and writes/echoes the per-row error report; exits non-zero if the file is rejected up front, zero when the run completes even with per-row errors (partial success is success — FR-018).

## Return shape: `ImportReport`

```
ImportReport {
    mode: ImportMode,
    summary: {
        clients:      { created, updated, skipped, errored },
        projects:     { created, updated, skipped, errored },
        tasks:        { created, updated, skipped, errored },
        time_entries: { created, updated, skipped, errored },
    },
    row_errors: [ { source_line, entity, reason }, ... ],   // FR-019
}
```

- Per entity type, `processed = created + updated + skipped + errored` MUST hold (FR-021, SC-005).
- In `DryRun` the counts are the would-create/update/skip/error preview and MUST match a subsequent `Commit` on the same unchanged input/data (FR-015, SC-004).

## Behavioral guarantees (cross-referenced)

- **FK-safe order**: clients → projects → tasks (+ `project_tasks`) → time entries (FR-004).
- **Idempotent**: matched rows are skipped/updated, never duplicated; a second identical run reports zero creations (FR-011). Natural keys per data-model.md.
- **Resilient**: a bad row is skipped and reported, run continues; each row's writes are all-or-nothing (FR-018, FR-020).
- **Exact**: durations → integer minutes, money → integer cents + ISO currency, no floats; full-file reconciliation to zero drift (FR-005/FR-006, SC-003).
- **User matching only**: users resolved by email; never provisioned; unmatched → row error (FR-010).
- **No invoice coupling**: entries import as `open`, never `invoiced` from Harvest's billed flag (FR-016).

## Out of scope (v1)

- OAuth2 Harvest REST API pull. Acknowledged as a future **source adapter** feeding the same engine; the mapping/matching core is source-agnostic so adding it does not rewrite this contract (research.md §1, §10).
