# Data Model: New Project

## General invariants

Every new table has UUID v7 `id` and `org_id` referencing organizations. Foreign entities are validated for the same organization inside mutations; composite foreign keys enforce tenant consistency where practical. New SQL functions revoke default PUBLIC execution and are reviewed against the plugin database allowlist. Private tables receive no plugin reader grants.

Amounts are nonnegative integer cents; durations integer minutes; percentages integer basis points (0–10,000). All aggregate arithmetic is checked. Typed creation settings are validated in core and again against database entities in server functions.

## Project draft

`project_drafts`: id, org_id, creator_id, revision (positive bigint), payload_version, payload (JSON object, \<=256 KiB), created_at, updated_at, completed_project_id nullable, discarded_at nullable.

Partial unique index on (org_id, creator_id) where unfinished and not discarded. States: absent → editing → completed OR discarded. Autosave compares expected revision; conflict returns no overwrite. Completed drafts are immutable and preserve retry identity. Finalize with current revision; a completed retry returns its project even if the submitted revision is stale. An old tab cannot recreate a discarded draft by saving.

Draft payload retains strings for partially typed amounts/dates and local stable row IDs. Bounds: 500 tasks, 500 people, 100 milestones, 50 tags, name 200/code 100/tag 50 characters. Client IDs can be absent while editing but must be active and same-org on commit. Do not echo confidential draft contents into logs.

## Project configuration

The shared editor projects operational rows into `ProjectForm` using one repeatable-read transaction after checking the current active manager/admin. `EditableProject` also carries the actual client and assigned catalog identities, including archived status, so pagination or archival cannot substitute defaults. A missing configuration remains `RateMode::Legacy`; reading does not create a settings row. Money, percentages and minutes are rendered from their exact stored integers, keeping absent overrides distinct from zero. Private notes and both project/profile cost values are redacted for managers. Milestones use their persisted IDs; existing tasks use catalog IDs as stable form-row identities.

Keep the current Project read model compatible. Add `project_settings` related 1:1 by unique project_id, with creator_id, rate_mode, budget_scope, monthly_reset, include_nonbillable, alert_enabled, alert_threshold, report_visibility, fee_mode/amount/monthly_day and invoice-default fields.

No settings row means legacy behavior. New rate modes: person/task/project; fixed and non-billable type governs whether hourly billing applies. Existing retainer rows remain legacy. For person mode assignment override falls back to compatible profile/client rate; task mode project-task override falls back to compatible catalog/client rate; project mode requires an explicit project rate. Snapshot default rates at creation where their currency cannot otherwise be represented unambiguously.

Dates/code stay in existing projects columns. Creation supports EUR/CHF/USD/GBP without altering legacy currency data.

`project_private_settings`: project_id unique, admin_notes (\<=10,000 characters). `project_member_costs`: project_id, user_id, cost_rate_cents with unique (project_id,user_id); administrator-only access. Avoid exposing these through existing Project/Assignment serialization or plugin grants.

Migration 0035 adds the private `project_read_access` view, keyed by current active user/org/project, with `can_view_rates`, `can_view_team` and `can_view_progress` flags. Organization managers/admins retain organization scope; assigned leads/admins can view progress independently of the visibility setting; ordinary assigned members require member-visible progress. A user's own historical time permits project identity only after assignment revocation. Missing settings preserve legacy assigned-member progress. The view has no PUBLIC grant and is not added to the plugin allowlist.

Migration 0036 adds private `task_read_access` for current active user/org/task identities, rate visibility and own-history membership. UI and compatibility reads share it; tracking authorization remains in `time_entry_contexts`. No PUBLIC or plugin grant is added.

## Existing-project edit concurrency

Migration 0039 adds `projects.edit_revision` and database triggers covering project rows and their settings, assignments, task links, private overrides, allocations, access links, tag links, milestones and fee occurrences. A changed operational row advances the revision; unchanged upserts do not. This covers existing detail actions and import writers as well as the shared editor, without requiring each caller to remember an invalidation step.

`project_edit_requests` records the UUID v7 request identity, organization, project, actor, expected/completed revisions and bounded form payload. It is private and receives no plugin grants. A committed retry is acknowledged only for the same actor/project/payload and unchanged completed revision; it never replays over a subsequent edit. The mutation rechecks current authority, locks the project, validates the graph and commits all changes and the request acknowledgement in one serializable transaction. Serialization/deadlock conflicts return a recoverable conflict, without partial writes.

Operational assignment/settings/access IDs survive upserts, and unchanged project-manager flags preserve existing Lead/Admin roles. Managers cannot write private values or indirectly erase a cost override by removing its assignment. Referenced tasks/people cannot be removed. Milestone position uniqueness becomes deferred so ordering can change without replacing persisted IDs; materialized invoice sources and charged milestone values cannot be rewritten. Invoice rows/lines and time entries are never updated by the editor.

## Tags

`project_tags`: reusable organization name with unique case-folded normalized key. `project_tag_links`: project_id/tag_id unique. Removing a link never deletes the shared tag. New draft tags are committed with the project, not on each keystroke. List/report filters reference tag identity and org.

## Tasks and team

`clients.default_rate_cents` is denominated in `clients.currency`, with zero distinct from absence. Existing client detail mutations cannot change that denomination without an explicit replacement amount; they lock and recheck the current row before updating. No migration or automatic conversion is performed, and clients without a default rate retain their previous editing behavior.

Reuse catalog tasks, project_tasks and assignments. Store new per-project task settings (restriction flag, budget minutes/cents) and allowed-user links in org-scoped relations. Restricted with an empty allowed set means nobody, not everyone. Default is unrestricted.

Additive migration 0038 adds nullable `tasks.default_rate_currency`, an uppercase three-letter source denomination. Existing catalog amounts remain unchanged with unknown currency; do not backfill from the workspace or linked projects. New CSV tasks retain a valid explicit source currency; API catalog tasks without source currency remain unknown. Native changes to the default amount use workspace currency; renames/no-ops preserve the prior denomination. Configured task-rate creation and enablement require an explicit project rate when a non-null catalog rate has unknown or incompatible currency, including zero. Compatible rates are snapshotted into project_tasks; legacy links and existing project amounts are unchanged. Preview checkpoints preserve this metadata, with older snapshots remaining unknown. Authorized creation options include the denomination so the UI cannot present an incompatible default as an inherited amount.

Reuse assignment roles for project lead designation; never change org_role. Per-person budgets are project configuration, not profile fields. All task/member references are active and same-org on creation. Removal from a draft also removes its scoped budget/access associations. Catalog task creation and project linking commit atomically; explicit client-dialog creation is an independent action.

Migration 0034 replaces the existing new-time context view with the task restriction predicate; it does not rewrite historical migrations. Current restriction rows and matching grants are share-locked during interactive time writes. Historical edits use these grants without the active-context filter, preserving archived work; stopping an own running timer deliberately bypasses the task restriction but retains ownership, open-state and submission guards. Empty restricted membership means no tracking access, including for organization administrators acting on their own timesheets.

## Fee schedule and invoice occurrences

The model below replaces exclusive whole-fee claims for FR-024/025. Migration 0040 removes the invoice pointer and stores `allocated_discount_cents` with checked bounds; `net_before_tax_cents` is a stored generated difference from the gross amount. Generation and draft edits replace these allocations using the core helper. Availability sums non-void contributions, including drafts. Existing-draft editing supports invoice-owned amounts/descriptions and explicitly confirmed overbilling; editable generation preparation and project context remain tracked by T063–T068.

`project_fee_milestones`: project_id, label, due_on, amount_cents, position. At most 100; required nonempty label, date and nonnegative amount. Single/monthly modes use project settings; monthly day is first/fifteenth/last, computed as a calendar date with leap-year tests.

`project_fee_occurrences`: project_id, milestone_id nullable, period_key, due_on, amount_cents, currency. Unique (project_id,period_key) identifies single, milestone or calendar-month occurrence. Materialize when explicitly generating the draft invoice; its read-only preparation preview does not create or reserve occurrences. Do not issue invoices on schedule automatically.

Occurrences also snapshot the original description. Keys are `single`, `milestone:<uuid>` or `month:<YYYY-MM>`. Composite foreign keys keep references in the same organization. Later partial invoices reuse the occurrence identity and agreed amount; invoice line descriptions may differ. A monthly balance belongs to its own month and cannot consume another month's fee.

Invoice lines allow exactly one of time_entry_id or fee_occurrence_id, enforced by CHECK. Existing time-backed rows retain their source. Fee rows do not fabricate minutes/hourly rates. Each line stores `net_before_tax_cents` between zero and its gross amount. Across an invoice these contributions sum to subtotal minus discount; taxes are excluded. Non-void contributions, including draft reservations, consume the occurrence; void leaves the amounts intact but excludes that invoice from balances. Index occurrence references for aggregation and retain uniqueness within each invoice.

Allocate the header discount proportionally across all gross lines using integer floors, then assign remaining cents by descending fractional remainder and stable source order. Preparation and persistence use the same ordering: time-entry UUID, then fee `(project_id, period_key)`. Zero subtotal means all contributions are zero. A forward migration computes contributions for existing lines without changing their gross values, headers or source identities, then removes the obsolete invoice pointer. No parallel legacy balance path is needed.

Invoice mutation requests retain UUID v7 identity, organization/actor, canonical bounded request, invoice identity and completed revision. Identical requests cannot create a second invoice or apply an edit twice; stale draft revisions conflict. Keep this private table outside plugin grants. All invoice writers share the organization advisory lock before invoice/occurrence row locks. Draft replacement validates balances excluding its own previous contribution, then atomically replaces lines, contributions, header and revision. Confirmed overbilling is bound to the reviewed balance and proposed net excess, never a blanket bypass flag.

Current generation requests use private `invoice_generation_requests` (id, org_id, actor_id, invoice_id, bounded JSON payload, creation time). A replay returns the same invoice in its current state without another creation event; changing actor or request values conflicts. The UI retains that identity and offers an explicit retry after an uncertain acknowledgement, disabling Cancel and input changes until recovery. Zero agreed fees retain their previous one-active-invoice behavior; a positive fee with a 100% discount retains its full balance.

Migration 0041 adds `invoices.edit_revision`, advanced by changed header or line rows, and private `invoice_edit_requests` with actor/invoice identity, completed revision and a canonical payload bounded to 256 KiB. Draft editing reads one consistent snapshot, edits existing fee lines without replacing their identities, and allocates discounts across both fee and frozen time rows. It replaces contributions and headers atomically under the invoice lock after checking current manager authority, expected revision, reviewed balances and exact per-line excess confirmations. A replay acknowledges only the same actor/invoice/payload at the completed revision; a later edit or status change rejects it. An inconsistent stored line subtotal is rejected rather than silently rewritten. Generation's editable source review and project balance context remain pending.

## Invoice-owned defaults and calculations

Add invoice snapshot fields: terms_days, po_number, discount_bps, tax1_bps, tax2_name/tax2_bps, subtotal_cents, discount_cents, tax1_cents, tax2_cents. Existing rows backfill zero adjustments and subtotal=total.

Additive migration 0037 introduces these without rewriting already-applied 0031. Stored generated columns derive `terms_days` from the invoice's own dates and `subtotal_cents` from its own total/adjustment components, never from project settings. This preserves existing dates and totals, including legacy writers that omit new fields. Numeric intermediates avoid overflow before the final bigint subtotal. Checks require the stored discount and each non-compounding tax to match exact half-up basis-point arithmetic; optional second-tax name/rate are paired.

Default terms 30 days; custom 0–365, dates checked for overflow. Tax/discount up to two decimal percentage places. Subtotal → rounded discount → discounted subtotal → separately rounded non-compounding taxes → checked total. Editing is draft-only. Mixed-project defaults require explicit input; unlike currencies cannot share an invoice.

## Budget notification

Use an org-scoped logical notification identity with project, scope entity, period key, integer threshold and recipient. A unique constraint prevents duplicate enqueue. Outbox payload references notification identity and contains no private rates/costs. Current recipients are creator plus project leads/org managers with project authority, deduplicated and active at enqueue/delivery.

Migration 0032 stores these in `project_budget_notifications`. Nullable task/user references identify scopes (both NULL means project); at most one is present. Composite foreign keys enforce organization ownership and `UNIQUE NULLS NOT DISTINCT` treats absent scope keys consistently. Period keys are `lifetime` or `YYYY-MM`. The transaction inserts outbox jobs only for newly inserted notification identities. This table remains outside plugin grants; historical identities are retained when consumption falls.

Reuse horae_outbox lease/token/backoff for the specific event kind; stable Message-ID across retries. Bound delivery attempts and record sanitized terminal failure. Acknowledged messages are not retried. Ambiguous external acknowledgement is documented at-least-once delivery.

Migration 0033 adds `horae_outbox.failed_at`, mutually exclusive with `delivered_at`. Claims and acknowledgement/failure updates exclude terminal rows. Budget email makes at most five send attempts, records permanent failure without claiming delivery, and retains the original notification identity. A missing configuration does not consume pending attempts. Other event kinds keep their existing retry policy.
