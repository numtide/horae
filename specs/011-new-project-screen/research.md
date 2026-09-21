# Research: New Project

## Draft and transactional creation

- **Decision**: One current private draft per creator/org, revision-checked saves, completed-draft identity retained for retry. Finalization locks and revalidates the draft and referenced rows before one transaction commits.
- **Rationale**: Existing `create_project` accepts seven fields and cannot atomically create the graph. The prototype save time is simulated.
- **Alternatives**: Browser-only persistence loses cross-session durability; inactive projects are genuine archived data, not drafts; sequential public calls can leave orphan associations.
- **Evidence**: `server_fns/projects.rs::{create_project,enable_project_task}`, `server_fns.rs::lock_project`, role-change serialization in `server_fns/users.rs`.

## Legacy financial compatibility

- **Decision**: Missing configuration means legacy task → assignment → project → user rate cascade. New rows select person/task/project modes; configured fixed fees do not become hourly charges. Zero remains explicit.
- **Rationale**: Task catalog rates are not a live legacy candidate. Existing fixed-fee projects are invoiced hourly; changing this silently would alter imported data.
- **Alternatives**: Global cascade replacement/backfill rejected; independent divergent formulas rejected in favor of a pure core helper and an SQL equivalent with parity tests.
- **Consumers**: `crates/core/src/invoice.rs::resolve_rate`, `server_fns/projects.rs::fetch_project_spend`, `server_fns/reports.rs::fetch_report`, `server_fns/invoices.rs::generate_invoice_for_period`, both Harvest time-entry queries, `importers/harvest/resolve.rs::resolve_project`.

## Exactness and currency

- **Decision**: Reuse `money::{parse_cents,add,format_cents_plain}`, `invoice::line_amount_cents`, duration and budget helpers. Add checked basis-point calculations using widened integers.
- **Rationale**: Helpers currently assume two decimals, and currency checks only constrain code length. Restrict new creation to reference EUR/CHF/USD/GBP; do not claim complete ISO minor-unit support. Existing reports group by client currency and have one currency for bill/cost totals; configured projects need separate billing/cost grouping and labels.
- **Alternatives**: Float arithmetic forbidden; new monetary dependency unnecessary; implicit cross-currency rate inheritance rejected.

## Fee invoicing and defaults

- **Decision**: Represent single/milestone/monthly fee occurrences with stable identity and invoice lines backed by either time or a fee. Lock/claim in transactions; voiding releases occurrences without rewriting historical lines. Prepare defaults explicitly before generation; issue/send remains manual.
- **Rationale**: `InvoiceLine.time_entry_id` is mandatory today; generation immediately creates a draft using Net 30 and client currency. Invoice defaults must be copied, not dynamically read from mutable projects.
- **Alternatives**: Fake time entries corrupt tracking; unused schedule JSON is not functional implementation; automatic invoice issuing is not requested.
- **Consumers**: `models/invoice.rs`, `server_fns/invoices.rs` and tests, `pages/invoices.rs`, `reports.rs`, `reports/streaming.rs`, `reports/limits.rs`, `templates/invoice.typ`.

## Budgets and notifications

- **Decision**: Keep BudgetKind denomination, add project/task/person scope, period and inclusion rules. Keep lifetime tracked totals separate. Uniqueness is project/scope/period/threshold/recipient; reuse filtered transactional outbox delivery.
- **Rationale**: Existing `check_project_budget` is lifetime hours, organization thresholds and fire-and-forget plugin events. It may reannounce after consumption drops and crosses again. It is not email.
- **Transport**: Optional absolute sendmail-compatible executable, direct spawn without shell, validated headers/addresses, bounded timeout/output and stable Message-ID. No transport means a disabled email control with explanation. Lost acknowledgement after acceptance permits duplicate receipt on retry; never claim exactly-once email.
- **Alternatives**: A new SMTP/TLS implementation, mandatory third-party account, bespoke delivery service or second queue is unnecessary.
- **Evidence**: `server_fns/budget_tests.rs`, `jobs.rs::{enqueue_outbox,claim_outbox,mark_outbox_delivered,mark_outbox_failed}`, `config.rs`, `scheduler.rs`. Current claim selects all event kinds and must be scoped.

## Permission boundaries

- **Decision**: Private draft/notes/cost tables are not added to plugin grants. Use purpose-specific projections and shared mutation guards. Project lead designation does not grant organization financial authority.
- **Rationale**: Existing project/task/spend and Harvest payloads expose financial fields. Plugins have table-level grants and a hard-coded relation/function allowlist. Adding private columns to granted tables would inherit those grants.
- **Alternatives**: CSS hiding is not authorization; using active-only eligibility for archived-history edits would regress historical work; a global auth rewrite is unnecessary.
- **Consumers**: `list_projects`, `list_project_spend`, `list_tasks`, `list_project_tasks`, `assignments_for_viewer`; project export count and stream queries; Harvest projects/tasks/time-entry payloads; plugin grants and event serialization.
- **Mutation seam**: migration `0019_time_entry_contexts.sql` covers new entry/timer starts and pickers. Update, reschedule, reorder and delete paths need the restriction predicate too. Own running timers may still stop after revocation; existing submission advisory barriers remain.
- **Tests**: `server_fns/projects/tests.rs`, `projects/{mutation_tests,bulk_tests}.rs`, `server_fns/test_seed.rs::wait_for_blocked`, `time_entries/update_tests.rs`, `approvals/submission_tests.rs`, Harvest and export tests.

## UI framework

- **Decision**: Existing AppLayout owns shell; use tokens/utilities plus only structural `np-*` rules. Reuse form controls, Modal, Checkbox, Radio, DatePicker and selectors; extend shared behavior opt-in with regression tests.
- **Rationale**: Existing components cover most visual primitives. Combobox needs names, disabled options and keyboard review before this large form relies on it. Native controls are preferable to additional dependencies.
- **Alternatives**: Pasted prototype HTML/inline styles, global CSS resets and fabricated data are rejected.
- **Evidence**: Full New Project reference, design system, `DESIGN.md`, `components/{form,controls,modal,combobox,date_picker}.rs`, `build.rs`.

## Navigation protection implementation seam

- Dioxus 0.7.9 `RouterConfig::on_update` runs after programmatic history changes; native `popstate` uses the history updater directly and does not call it. A redirect-only callback would therefore not protect browser Back and could overwrite a history entry.
- Pending-navigation work must cover link/programmatic navigation, browser Back/Forward and full-document unload, with cancellation preserving the form and browser history. Do not substitute a `beforeunload` listener alone or change shared routing without focused history tests. The existing explicit Cancel/back buttons already serialize a final draft save.

## Interrupted response-body decoding

- Local Dioxus fullstack 0.7.9 panics at `magic.rs` when `res.bytes()` fails after headers arrive. Initial hard navigations in the browser suite reproduced this; failed requests before headers are already returned to the form's retry path.
- Upstream [commit c64415c](https://github.com/DioxusLabs/dioxus/commit/c64415c08f7cef3c26cb8d3ab33985a258476a44) replaces the unwrap with error propagation. The latest published release was checked through the GitHub API on 2026-09-21: **v0.7.10 still contains the unwrap**. Updating only to that release would not resolve it. No dependency update has been made.
- Final recovery verification needs a deterministic truncated-body test and a compatible fix/backport; successful teardown sequencing is not proof that arbitrary network interruptions are safe. Keep T019/T051 open until this path is handled without freezing the form.
- The upstream fix's workspace is `0.8.0-alpha.0`, so pointing Cargo directly at that revision is not a compatible 0.7 patch. A local backport of the single-line change to the published 0.7.9 crate is awaiting approval; no dependency source or version has changed.
- `new-project-transport.cjs` now deterministically reproduces the body-read panic on the current build. It forwards a real save, preserves its successful headers and URL, then injects a failing response stream and verifies that the body reader was reached. Preserving the URL is essential: a synthetic Response's empty URL otherwise causes an earlier, recoverable request error and a misleading green test. The expected behavior remains error status, identical retry, subsequent newer edits and successful reopen without a WASM panic.

## Clarification coverage

No extra questions were required after the full workflow request. Scope, lifecycle, security, interaction, reliability, dependencies, edge cases, terminology and completion criteria are covered with explicit assumptions in the spec; these are not fabricated user answers. Runtime mail configuration is not an architectural unknown. Research corrected two overstrong inferred guarantees: email receipt and plugin delivery are not exactly-once external effects.
