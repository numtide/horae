# Importer UI Contract

Supplement features 005–007 without changing execution, authorization, retention or account binding.

## Connection matrix

| State | Presentation/actions |
|---|---|
| Loading | Neutral checking state; do not assume API availability. CSV is independently available. |
| Unavailable | Status unavailable and retry check; no false disconnected/unconfigured badge. |
| Unconfigured | Deployment setup required; offer CSV, not a secret editor. |
| Configured, unbound | Connect Harvest primary; explain read-only Harvest access. |
| Connected | Bound identity, actual status, Manage connection; group Disconnect and Change account inside. |
| Expired | Expired state and reconnect/recheck guidance; no fixed-time refresh promise. |
| Disconnected, bound | Retained binding, Reconnect original account and eligible or explained-blocked Change account. |

Keep the stable native confirmation component, captured account/version, cancellation, busy state and recovery. Never release/change a connection during rendering. Imported-data/active-work blockers remain visible before confirmation.

## Workflow/result matrix

| State | Next action/message |
|---|---|
| Source ready | Preview; business records remain unchanged. |
| Current completed preview | Confirm primary; new preview secondary. |
| Historical preview | New preview; cannot directly confirm historical results. |
| Queued | Waiting to start; cancellation may be offered. |
| Running | Readable phase/counts and cancellation; leaving page does not stop work. |
| Cancelling | Request accepted, not completed; prevent repeat cancellation. |
| Monitoring unavailable | Resume same ID; work may continue. Never submit a replacement implicitly. |
| Failed/cancelled with report | Partial confirmed-batch result; retry eligibility does not change completeness. |
| Failure without report | Actionable failure, no success banner. |
| Completed import | Exact outcomes; explicit zero/all-skipped detail, never unconditional “Every record was written”. |
| No selected report | Selection/preview guidance, not lifetime import history. |
| No retained history | No retained imports; no inference about business-data absence. |

At most one workflow-primary action per state, excluding shell controls and the active modal. Preserve current-preview origin guards on source replacement, account changes, historical selection and reload. Navigation must not cancel accepted work.

## History/progress

Humanize API/CSV source and known Preview/Import mode; neutral fallback for unknowns. Render readable timestamps with explicit timezone (existing chrono/UTC is sufficient) and machine-readable value. Distinguish selected rows semantically and visually. Preserve organization-scoped pagination, refresh and retained reports.

Humanize known state/phase strings; unknowns remain neutral. Show observed counts. Unknown or non-comparable totals use indeterminate presentation, not fabricated percentage/ETA. Announce meaningful state/phase changes politely, not every poll.

## Retry snapshot compatibility

Add only `JobStatus.retry_availability` as defined in [data-model.md](../data-model.md). Fill it consistently through the projection used by status/history and start/cancel/retry replies. Unknown kind/version yields unknown; only recognized retryable work gets available. API generation fencing does not apply to CSV.

Old clients ignore the additive field; new readers default absent/future values to unknown. Preserve existing fields, HTTP paths, requests, kind/state wire strings, CLI validation and exit codes. Do not expose payloads, upload bytes, private cursors or credentials.

Unknown availability disables Retry with refresh/update guidance. Previous account and missing upload show a reason but retain report/error access. Stale available snapshots may be rejected: show the error and refresh, with no automatic resubmission. The write-side retry gate remains authoritative. Do not redefine `JobStatus::can_retry()` or base partial-result presentation on availability.

Update feature-005/006 contract documentation and SQLx cache during implementation when the projection changes.

## Errors, layout and accessibility

Identify the failed action and known recovery; unknown errors get neutral guidance rather than brittle substring-based classification. Technical detail is secondary/expandable, escaped, and must not introduce tokens, secrets, session data or raw payload exposure. Keep complete error downloads and their actual NDJSON format.

Reuse native Modal focus/inertness/dismissal behavior. Connection disclosure exposes expanded/control association; selected history has accessible state. Action errors use alerts without repeatedly moving focus. Pending actions disable repeat submission and unsafe dismissal.

Essential controls fit 360/768/1440 widths, 640 height and 200% zoom. Long identifiers wrap or expose an accessible full value; wide error tables scroll locally, not page-wide. Status uses text as well as color.

## Design alignment and deviations

Use `design/project/app/10_Importers.dc.html` with `DESIGN.md`; keep the existing app/admin shell. Use tokens/utilities and semantic components, not copied inline styles, hardcoded palette values or prototype developer controls.

- Adapt durable history, queued/cancelling/monitoring-unavailable states and Change account to the handoff's visual vocabulary.
- Preserve the actual OAuth redirect flow; do not promise a popup that does not open.
- No OAuth-secret editor, handshake tester or per-user connection model.
- No fake account names, last-sync times or fixed-time automatic token repair.
- Preserve supported CSV headers/limits; do not promise arbitrary tracker compatibility or mock limits.
- Preserve NDJSON errors rather than copying the prototype's CSV download label.
- No redesign of global navigation or other surfaces.

Record concrete deviations and browser evidence during implementation in `acceptance.md`.
