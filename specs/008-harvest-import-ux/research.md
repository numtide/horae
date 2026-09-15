# Research: Harvest Import UX

## Baseline and limits

Planning starts at `d32d56f` on master. The preceding source comparison covered `pages/importers.rs` and `design/project/app/10_Importers.dc.html`; it was not browser/pixel acceptance. The implementation audit must read the complete handoff and required support files using the design skill. This planning investigation made no source, database or provider changes.

## Decisions

### Preserve the coordinator

**Decision**: Keep current source selection, watching, pagination and request guards; add small display helpers.

**Rationale**: Existing tests cover durable work and stale results. Labels, action placement and false empty/success messages can change without replacing execution semantics.

**Alternatives**: A generic wizard/global store is unnecessary; CSS-only work cannot correct misleading copy or eligibility.

### Reuse connection data

**Decision**: Reuse `ConnectionStatus`, `change_account_blocker()` and native Modal; group Disconnect and Change account in management. Display the identifier when no verified name exists.

**Rationale**: Configuration, binding, expiry, generation/revision, provenance and active count already exist.

**Alternatives**: A provider lookup, OAuth-secret editor, fake last-sync time or fixed one-minute refresh promise would expand scope or misrepresent observed data.

### Add only retry availability

**Decision**: Reuse report source/mode and add a defaulted read-only `retry_availability` to `JobStatus`.

**Evidence**: `jobs.rs::JobPayload::initial_report()` initializes API and CSV reports at enqueue, before execution. `models/jobs.rs::JobStatus::can_retry()` checks failed/cancelled state only. `jobs.rs::retry()` separately checks API generation and CSV upload existence. ConnectionStatus needs no extension.

**Rationale**: Existing report mode supports normal history; missing legacy mode can remain unknown. Actual retry prerequisites cannot be inferred from the public snapshot today.

**Alternatives**: Expose payloads, stored account IDs or raw generations; add per-row requests; infer Commit from a source kind. None is needed.

### Separate completeness from retry

**Decision**: Do not redefine `can_retry()`. Classify interrupted reports from terminal state, and use the new availability only for the retry affordance.

**Evidence**: `pages/importers.rs` currently uses that helper for both buttons and partial-result banners. Making it generation-aware would incorrectly make old-account partial results appear complete.

**Alternatives**: Global helper replacement is superficially smaller but changes existing UI/CLI semantics. An explicit independent snapshot avoids that regression.

### Prefer truthful observations

**Decision**: Distinguish no selection, no retained history and zero processed records. Keep created/updated/skipped/errored outcomes distinct. Show counts and unknown totals without invented ETA/percentage.

**Rationale**: Empty selection/history does not prove no imported business data; previews still create operational job/report records. A completed import can legitimately skip every row.

**Alternatives**: Add a lifetime counter just to retain “Nothing imported yet”, or copy mock progress values. Both are unnecessary.

### Reuse browser-test conventions

**Decision**: Add `tests/browser/importers.cjs` following `tests/browser/modals.cjs`: explicit isolated URL, role selectors, fixture scenarios and actual focus checks.

**Rationale**: Dioxus state tests cannot prove browser layout/focus. Existing browser conventions avoid adding an application dependency.

**Alternatives**: Count compilation as visual acceptance or test destructive scenarios against the real account. Neither is acceptable.

## Compatibility notes

Old clients ignore additive fields; new readers need a default/unknown fallback. Preserve `cli/imports.rs::decode_job()` kind/state validation and exit behavior. Update constructors in `tests/import_jobs_ui.rs` and `src/cli/imports/tests.rs`, test status/list projection and regenerate SQLx metadata.

CSV retries are not fenced by Harvest generation. Same-generation API retry can currently be accepted while disconnected: offer reconnect guidance without claiming a new backend rejection. No planning unknown remains unresolved.
