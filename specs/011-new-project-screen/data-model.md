# Data Model: New Project

## General invariants

Every new table has UUID v7 `id` and `org_id` referencing organizations. Foreign entities are validated for the same organization inside mutations; composite foreign keys enforce tenant consistency where practical. New SQL functions revoke default PUBLIC execution and are reviewed against the plugin database allowlist. Private tables receive no plugin reader grants.

Amounts are nonnegative integer cents; durations integer minutes; percentages integer basis points (0–10,000). All aggregate arithmetic is checked. Typed creation settings are validated in core and again against database entities in server functions.

## Project draft

`project_drafts`: id, org_id, creator_id, revision (positive bigint), payload_version, payload (JSON object, \<=256 KiB), created_at, updated_at, completed_project_id nullable, discarded_at nullable.

Partial unique index on (org_id, creator_id) where unfinished and not discarded. States: absent → editing → completed OR discarded. Autosave compares expected revision; conflict returns no overwrite. Completed drafts are immutable and preserve retry identity. Finalize with current revision; a completed retry returns its project even if the submitted revision is stale. An old tab cannot recreate a discarded draft by saving.

Draft payload retains strings for partially typed amounts/dates and local stable row IDs. Bounds: 500 tasks, 500 people, 100 milestones, 50 tags, name 200/code 100/tag 50 characters. Client IDs can be absent while editing but must be active and same-org on commit. Do not echo confidential draft contents into logs.

## Project configuration

Keep the current Project read model compatible. Add `project_settings` related 1:1 by unique project_id, with creator_id, rate_mode, budget_scope, monthly_reset, include_nonbillable, alert_enabled, alert_threshold, report_visibility, fee_mode/amount/monthly_day and invoice-default fields.

No settings row means legacy behavior. New rate modes: person/task/project; fixed and non-billable type governs whether hourly billing applies. Existing retainer rows remain legacy. For person mode assignment override falls back to compatible profile/client rate; task mode project-task override falls back to compatible catalog/client rate; project mode requires an explicit project rate. Snapshot default rates at creation where their currency cannot otherwise be represented unambiguously.

Dates/code stay in existing projects columns. Creation supports EUR/CHF/USD/GBP without altering legacy currency data.

`project_private_settings`: project_id unique, admin_notes (\<=10,000 characters). `project_member_costs`: project_id, user_id, cost_rate_cents with unique (project_id,user_id); administrator-only access. Avoid exposing these through existing Project/Assignment serialization or plugin grants.

## Tags

`project_tags`: reusable organization name with unique case-folded normalized key. `project_tag_links`: project_id/tag_id unique. Removing a link never deletes the shared tag. New draft tags are committed with the project, not on each keystroke. List/report filters reference tag identity and org.

## Tasks and team

Reuse catalog tasks, project_tasks and assignments. Store new per-project task settings (restriction flag, budget minutes/cents) and allowed-user links in org-scoped relations. Restricted with an empty allowed set means nobody, not everyone. Default is unrestricted.

Reuse assignment roles for project lead designation; never change org_role. Per-person budgets are project configuration, not profile fields. All task/member references are active and same-org on creation. Removal from a draft also removes its scoped budget/access associations. Catalog task creation and project linking commit atomically; explicit client-dialog creation is an independent action.

## Fee schedule and invoice occurrences

`project_fee_milestones`: project_id, label, due_on, amount_cents, position. At most 100; required nonempty label, date and nonnegative amount. Single/monthly modes use project settings; monthly day is first/fifteenth/last, computed as a calendar date with leap-year tests.

`project_fee_occurrences`: project_id, milestone_id nullable, period_key, due_on, amount_cents, currency, invoice_id nullable. Unique (project_id,period_key) identifies single, milestone or calendar-month occurrence. Materialize on explicit invoice preparation; do not issue invoices on schedule automatically.

Occurrences also snapshot the line description. Keys are `single`, `milestone:<uuid>` or `month:<YYYY-MM>`. Composite foreign keys keep project/milestone/invoice references in the same organization. Claimed occurrences remain available as historical invoice sources after a void releases their current claim; later generation reuses the occurrence identity and frozen amount.

Invoice lines allow exactly one of time_entry_id or fee_occurrence_id, enforced by CHECK. Existing time-backed rows retain their source. Fee rows do not fabricate minutes/hourly rates. Invoice transaction claims available occurrences; void releases claims but leaves original invoice snapshots immutable.

## Invoice-owned defaults and calculations

Add invoice snapshot fields: terms_days, po_number, discount_bps, tax1_bps, tax2_name/tax2_bps, subtotal_cents, discount_cents, tax1_cents, tax2_cents. Existing rows backfill zero adjustments and subtotal=total.

Default terms 30 days; custom 0–365, dates checked for overflow. Tax/discount up to two decimal percentage places. Subtotal → rounded discount → discounted subtotal → separately rounded non-compounding taxes → checked total. Editing is draft-only. Mixed-project defaults require explicit input; unlike currencies cannot share an invoice.

## Budget notification

Use an org-scoped logical notification identity with project, scope entity, period key, integer threshold and recipient. A unique constraint prevents duplicate enqueue. Outbox payload references notification identity and contains no private rates/costs. Current recipients are creator plus project leads/org managers with project authority, deduplicated and active at enqueue/delivery.

Reuse horae_outbox lease/token/backoff for the specific event kind; stable Message-ID across retries. Bound delivery attempts and record sanitized terminal failure. Acknowledged messages are not retried. Ambiguous external acknowledgement is documented at-least-once delivery.
