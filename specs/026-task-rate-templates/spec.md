# Feature Specification: Task rate templates

**Feature Branch**: `spec/task-rate-templates` (documents). The implementation comes after the
Harvest invoice import (spec 025) and the next deploy, in two PRs: the rounding change first, then
the templates

**Created**: 2026-11-22

**Status**: Clarified (2026-11-23)

**Input**: User description: "Change a task's default rate from money with its own currency into a
currency-free rate template that takes the project's currency when it is used, as Harvest does."

Supersedes the catalog denomination decision of [011 New Project screen](../011-new-project-screen/research.md)
("Catalog denomination", migration 0038). Builds on the "Unknown currency" fix (PR 1, branch
`fix/unknown-currency`, migration `0009_zero_task_rates.sql`), which must merge first.

## Background

Tasks are shared by the whole workspace, and one task is used in projects billed in different
currencies. Today a task's default rate is money: an amount plus a currency. The task editor stamps
the workspace currency on every amount it saves. The Harvest API importer stores Harvest's
`default_hourly_rate` with no currency, because Harvest's task has none. When a task is added to a
project, New Project and the project editor copy the default rate only when its currency matches the
project's, and otherwise ask for an explicit rate.

Spec 011 chose that rule after an imported 80 USD task rate turned into 80 EUR in New Project. Its
principle was that a missing currency is not the workspace currency. In practice the rule makes
every imported default rate unusable (Harvest gives none of them a currency), and makes any task
used across currencies need a rate typed by hand in every project.

Harvest treats a task's default rate as a plain number with two decimals. Adding the task to a
project gives it that number as a rate in the project's currency. In the Harvest test account, a
JPY project with a task rate of 80.50 billed 1.5 hours as JPY 121 (120.75 rounded half up) and 0.5
hours at 5.00 as JPY 3 (2.50 rounded up). Its invoice subtotal was 123, because Harvest rounds the
unrounded sum (123.25) rather than adding the rounded lines.

Horae stores every amount in hundredths of the currency's major unit, whatever the currency: JPY 121
is stored as 12100 and BHD 1.25 as 125 (`horae_core::money::format_major_units`). A template with two
decimals therefore has the same integer as the rate it becomes in any project currency. Applying a
template never rescales a number; only the rounding of amounts computed from a rate depends on the
currency.

## Clarifications

### Session 2026-11-23

- Q: Is a template of 0.00 a valid template that applies a zero rate, or is 0 "no rate"? → A: 0 is
  no rate (FR-006).
- Q: Does the rounding apply to invoice lines only, or to every amount computed per time entry? →
  A: every per-entry amount: invoice previews and lines, uninvoiced amounts, reports, project and
  budget figures. Amounts already stored on invoices do not change (FR-011).
- Q: A task rate stored in a currency other than the workspace's: keep the number or clear it? →
  A: keep the number, and list the affected tasks in the upgrade notes (FR-012).
- Q: Is the constitution amendment approved, and at which version? → A: approved as a MINOR change,
  7.2.0, applied after the OAuth amendment (7.1.0) lands (FR-022).
- Q: Should Principle I's "minor units (cents)" wording be fixed here? → A: no. It is a separate
  PATCH later, outside this spec (see Follow-up).
- Q: One PR, or the rounding change first? → A: two deliveries. The rounding change (User Story 3)
  ships first as its own smaller PR, and the templates follow (plan.md, Delivery).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A task's default rate fills projects in any currency (Priority: P1)

A project manager adds the task "Design", whose default rate is 80.50, to a new JPY project and to an
existing EUR project. Each project gets a task rate of 80.50 in its own currency without anyone
typing it, and the screen shows the rate the project will bill ("JPY 80.50 / h", "EUR 80.50 / h")
before it is saved.

**Why this priority**: Today every imported default rate and every rate used across currencies is
refused here, so this is the change people feel.

**Independent Test**: Give a task a default rate, add it to projects in EUR, USD and JPY from New
Project and from the project editor, and check that each project's task rate equals the template in
the project's currency.

**Acceptance Scenarios**:

1. **Given** a task with the default rate 80.50 and a New Project draft billed in JPY by task rate,
   **When** the manager adds the task, **Then** its rate field shows 80.50 with the label in JPY, and
   saving the project stores the task rate JPY 80.50.
1. **Given** the same task and an existing EUR project in the project editor, **When** the manager
   adds the task and leaves its rate empty, **Then** the project's task rate is EUR 80.50 and no error
   asks for an explicit rate.
1. **Given** a task with no default rate, **When** it is added to any project, **Then** the task rate
   stays empty, as today.
1. **Given** a project that already has the task at EUR 70.00, **When** someone changes the task's
   default rate to 90.00, **Then** the project keeps EUR 70.00 (the copy made when the task was added
   is the project's own rate).
1. **Given** a manager who may not read billable rates, **When** they add a task, **Then** the
   template is applied without showing its number, as the existing rate visibility rules require.

______________________________________________________________________

### User Story 2 - The task catalog shows templates without a currency (Priority: P1)

An administrator opens Tasks. Each task's default rate reads as a number per hour with no currency,
for example "80.50 / h", with wording that says it takes each project's currency. The task editor's
rate field has no currency and accepts at most two decimals.

**Why this priority**: The catalog is where the template is set; it must stop claiming a currency it
no longer has.

**Independent Test**: Create and edit tasks with rates 80, 80.5, 0.01 and an empty rate, and check
what the Tasks screen and the editor show and store.

**Acceptance Scenarios**:

1. **Given** the task editor, **When** an administrator enters 80.5, **Then** the task's template is
   80.50 and the Tasks screen shows "80.50 / h" with the "each project's currency" wording.
1. **Given** the task editor, **When** someone enters 80.505, **Then** the form refuses it as more than
   two decimals.
1. **Given** a workspace whose currency changes from EUR to USD, **When** an administrator then edits
   a task's name only, **Then** its template is unchanged and no "currency changed, reload" refusal
   appears.
1. **Given** a task with the template 80.00, **When** an administrator enters 0, **Then** the task
   has no rate and the Tasks screen shows "No rate" (FR-006).

______________________________________________________________________

### User Story 3 - Invoice and report amounts round to the currency's decimals (Priority: P2)

A JPY project bills 1.5 hours at JPY 80.50. Its uninvoiced amount, the invoice line and the report
all show JPY 121, and an invoice with that line and a 0.5 hour line at JPY 5.00 totals JPY 124: the
sum of its lines, 121 + 3.

**Why this priority**: Once templates fill projects in any currency, fractional rates in
zero-decimal currencies become common, and amounts must not carry fractions the currency does not
have. It does not depend on templates, so it ships first, as its own PR (plan.md, Delivery).

**Independent Test**: Preview and create an invoice in JPY with the two lines above, and compare the
lines, the invoice total and the detailed report.

**Acceptance Scenarios**:

1. **Given** a JPY project-task rate of 80.50 and an entry of 90 minutes, **When** the invoice is
   previewed, **Then** the line amount is JPY 121.
1. **Given** a JPY rate of 5.00 and 30 minutes, **Then** the line amount is JPY 3.
1. **Given** both lines on one invoice, **Then** its subtotal is JPY 124, the exact sum of the
   rounded lines, unlike Harvest's 123.
1. **Given** the same entries in EUR, **Then** the lines are EUR 120.75 and EUR 2.50, as today.
1. **Given** invoices created before this change, **Then** their stored line amounts and totals do
   not change.

______________________________________________________________________

### User Story 4 - Imports keep Harvest's number (Priority: P2)

An administrator imports a Harvest account. Each task's `default_hourly_rate` becomes its template as
is (80.5 → 80.50). A Harvest rate of 0 becomes no rate, as PR 1 already does. The CSV import stores a
task's template from the row's rate without a currency, and keeps checking each project-task rate
against the project's currency, because that rate is real money in the row's currency.

**Why this priority**: Imports are where most catalogs come from, and they are the reason the old
rule failed.

**Independent Test**: Run the Harvest API and CSV importers on fixtures with task rates of 80.5, 0
and none, and project-task rates in the project's currency and in another currency.

**Acceptance Scenarios**:

1. **Given** a Harvest task with `default_hourly_rate` 80.5, **When** it is imported, **Then** the
   task's template is 80.50.
1. **Given** a Harvest task with `default_hourly_rate` 0 or null, **Then** the task has no template.
1. **Given** a CSV row whose task is new and whose rate is 75.00 USD, **Then** the task's template is
   75.00, and no currency is stored on the task.
1. **Given** a CSV row whose project-task rate is in a currency other than the project's, **Then**
   the row is refused as today.
1. **Given** an import preview checkpoint written before this change, **When** the commit reads it,
   **Then** it still commits.

______________________________________________________________________

### User Story 5 - Existing rates move to templates without surprises (Priority: P2)

An administrator upgrades Horae. Every task default rate becomes a template with the same number;
the stored currencies are dropped. Project-task rates, client rates, invoices and reports do not
change.

**Why this priority**: The data must survive the change, and nobody should find an invoice or a
project rate repriced.

**Independent Test**: Migrate a database with task rates in the workspace currency, in another
currency, with no currency, and of zero, and compare every project-task rate, invoice and report
before and after.

**Acceptance Scenarios**:

1. **Given** a task rate of EUR 80.00 in an EUR workspace, **Then** its template is 80.00.
1. **Given** a task rate with no currency (imported), **Then** its template is the same number.
1. **Given** a task rate of USD 80.00 in an EUR workspace, **Then** its template is 80.00, and the
   upgrade notes' query lists the task before the upgrade (FR-012).
1. **Given** a zero task rate with a currency (entered in the editor), **Then** the task has no rate
   (FR-006).
1. **Given** any project, **Then** its task rates, invoices and report totals are identical before
   and after the migration.

______________________________________________________________________

### Edge Cases

- A template in a three-decimal currency (BHD, KWD): applied, it is the same number of hundredths;
  amounts computed from it round to hundredths, the finest unit Horae stores (FR-009).
- A template applied to a JPY project with hundredths (80.50) stays 80.50: Harvest keeps two-decimal
  rates in every currency, and only the amounts round.
- A rate times a duration that is exactly half a unit (2.50 JPY) rounds up.
- An amount that is close to a half after a first rounding (120.496 → 120.50 → 121) must round once,
  from the exact value, to JPY 120 (FR-008).
- The largest template that fits, applied and multiplied by the longest entry, either fits or is
  refused as out of range; it never wraps.
- A task added in New Project to a project whose currency the manager changes afterwards: the rate
  shown follows the new currency, since the number is the same.
- A task is renamed or archived: its template is untouched.
- A person who cannot read billable rates sees neither templates nor the applied rate, as today.

## Requirements *(mandatory)*

### Functional Requirements

**Model**

- **FR-001**: A task's default rate MUST be a rate template: a positive number with at most two
  decimals and no currency, or no rate. It is not money and MUST NOT be summed, reported or invoiced
  as money.
- **FR-002**: Horae MUST NOT store, infer or display a currency for a template. It MUST NOT guess a
  currency anywhere: an applied rate takes the project's currency, which every project has.

**Applying a template**

- **FR-003**: When a task is added to a project and the person leaves its rate empty, the project's
  task rate MUST become the template's number in the project's currency, in New Project, in the
  project editor, and wherever else a task is attached to a project with its default rate. The
  refusals "currency is unknown or incompatible" and "Enter an explicit task rate in the project
  currency" MUST be removed.
- **FR-004**: A rate that the person types MUST win over the template, as today.
- **FR-005**: A project's task rate MUST stay a copy: changing or clearing the template later MUST
  NOT change any project's rate.
- **FR-006**: A template of zero MUST mean no rate, as Harvest's API and PR 1's import repair
  already treat it: the task editor stores no rate when 0 is entered, and upgrading clears stored
  zero rates. A project can still set 0.00 as its own task rate.
- **FR-007**: Wherever a template is about to be applied (New Project's task rows, the project
  editor's task rows), the screen MUST show the resulting rate in the project's currency, for example
  "EUR 80.50 / h", before the project is saved.

**Rounding**

- **FR-008**: An amount computed from an hourly rate and a duration MUST be rounded once, half up,
  from the exact value (rate × minutes ÷ 60) to the currency's precision: whole units for a
  zero-decimal currency, hundredths for every other currency.
- **FR-009**: For a three-decimal currency the precision is hundredths, because Horae stores
  hundredths for every currency; amounts stay exact in that currency.
- **FR-010**: An invoice's subtotal and total MUST equal the exact sum of its rounded lines, as spec
  001 FR-023 requires, even where Harvest's subtotal differs.
- **FR-011**: The rounding of FR-008 MUST apply to every amount computed per time entry: invoice
  previews and lines, uninvoiced amounts, reports, and project and budget figures, so a report and
  the invoice for the same time agree. Amounts already stored on invoices MUST NOT change.

**Migration**

- **FR-012**: Upgrading MUST turn every stored task default rate into a template with the same
  number, apart from zero rates (FR-006). A rate whose stored currency is not the workspace currency
  MUST keep its number, as Harvest would (80 USD becomes the template 80.00, applied as 80.00 in an
  EUR project), and the upgrade notes MUST give a query that lists those tasks before upgrading.
- **FR-013**: Upgrading MUST NOT change project-task rates, client, person or project rates,
  invoices, or any report figure other than through FR-011.

**Catalog and editor**

- **FR-014**: The Tasks screen MUST show a template as a number per hour without a currency, with
  catalog wording that it takes each project's currency, and "No rate" when there is none. PR 1's
  "currency not set" wording MUST go.
- **FR-015**: The task editor MUST label the field as a default hourly rate without a currency,
  accept at most two decimals, and stop checking the workspace currency when it saves.

**Imports**

- **FR-016**: The Harvest API importer MUST store a task's `default_hourly_rate` as its template
  unchanged, and no rate for 0 or none.
- **FR-017**: The CSV importer MUST store a new task's template from the row's rate without a
  currency, and MUST keep its per-row currency check for project-task rates.
- **FR-018**: Import preview checkpoints written before the upgrade MUST still commit.

**Interfaces**

- **FR-019**: The Harvest-compatible API MUST keep the shape of a task: `default_hourly_rate` is the
  template written with two decimals, or null.
- **FR-020**: `/api/v1` MUST NOT change; it exposes no task rates, and its OpenAPI snapshot stays
  identical.
- **FR-021**: The plugin event for a created task keeps its `default_rate_cents` field, now the
  template in hundredths; the field never had a currency.

**Governance**

- **FR-022**: The constitution's Principle I MUST state that a rate template is not money until it
  is applied, and that money stays integer minor units with an ISO currency. The approved amendment
  (MINOR, 7.2.0) is in `constitution-amendment.md`, and the templates PR applies it as its first
  change, after the OAuth amendment (7.1.0) has landed.
- **FR-023**: Spec 011's catalog denomination notes MUST be marked as superseded by this spec.

### Key Entities

- **Rate template**: a task's default hourly rate, in hundredths, with no currency. Belongs to one
  task; optional.
- **Project task rate**: the rate a project bills for a task, in hundredths of the project's
  currency. Copied from the template when the task is added, or typed. Unchanged by this feature.
- **Currency precision**: the unit an amount rounds to: one whole unit for a zero-decimal currency,
  one hundredth otherwise.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Adding a task with a default rate to a project in any currency needs no typed rate:
  zero refusals for "currency is unknown or incompatible" in New Project and the project editor.
- **SC-002**: After an import from Harvest, 100% of tasks with a non-zero Harvest default rate fill
  the rate of every project they are added to.
- **SC-003**: For every invoice, the subtotal equals the sum of its lines, checked over generated
  cases in all three currency precisions.
- **SC-004**: A JPY invoice line for 1.5 hours at 80.50 reads JPY 121 in the preview, the invoice,
  the PDF and the report.
- **SC-005**: Upgrading changes no project-task rate, invoice amount or report total in a 2-decimal
  currency (checked before and after on a copy of a real database).
- **SC-006**: Nowhere in the interface does a template appear with a currency, and nowhere does an
  applied rate appear without one.

## Assumptions

- PR 1 (`fix/unknown-currency`) has merged: Harvest's 0 is no rate, migration `0009` cleared stored
  zeros without a currency, and the Tasks screen says "currency not set".
- Money storage stays as it is: hundredths of the major unit plus an ISO code for every currency.
  Storing thousandths for three-decimal currencies is out of scope.
- New Project still restricts new projects to the reference currencies of spec 011; imported projects
  can have any currency.
- Client default rates keep their currency and their existing rule (they apply only to projects in
  the client's currency). Person rates are unchanged.
- The migration has to be correct, not reversible: the current deployments can be reset and
  re-imported if needed.
- Harvest's three-decimal behaviour was not tested; FR-009 follows from Horae's storage.

## Follow-up (outside this spec)

- Principle I says money is "integer minor units (cents)", but Horae stores hundredths of the major
  unit for every currency, which are not ISO minor units for JPY or KWD. A separate PATCH of the
  constitution fixes that wording later; this spec and its amendment leave it as it is.
