# Feature Specification: Interface translation (i18n)

**Feature Branch**: `spec/i18n`

**Created**: 2026-11-20

**Status**: Draft; awaiting the maintainer's review before `/speckit-plan`

**Input**: User description: "Internationalize Horae. The first step moves every interface text
into a catalog, English only, without changing what anyone sees, so that adding a language later
does not touch the screens."

Research: [research.md](research.md) summarises the study and the proof of concept behind the
decisions below.

## Background

Every word Horae shows is written inline in the code: about 2,000 texts in the interface's
91 files, plus the sign-in page, the budget email, the invoice PDF, the export headers and the
error messages of the server. Translating any screen means editing that screen, and nothing stops
a new screen from adding more inline text.

This feature puts the interface's text in one catalog per language and defines how Horae picks
the language for each reader. It is delivered in increments. The first one moves the interface's
text into an English catalog and changes nothing a person can see. The later ones choose the
language, translate the server's messages and the documents Horae produces, and make each
language an addition to the catalog.

## Constraints (maintainer decisions, 2026-11-20)

These were decided before this specification and are not reopened here.

- **C-1 Catalog and library.** Texts live in Fluent catalogs (`.ftl`), one set per language. A
  dedicated crate, `horae-i18n`, reads them. It depends on neither Dioxus nor Axum, builds for
  the server and for WASM, and generates one method per message, so code that names a message
  that does not exist, or omits one of its arguments, does not compile. This is the design of the
  proof of concept on `study/i18n`. The cost it adds to the WASM bundle (about 42 KB brotli,
  +3 %) is accepted.
- **C-2 Interface language.** In order: the person's own preference; if they have none, the
  organization's default language; and only if the organization sets none, the browser's
  language (`Accept-Language`). Before sign-in, the browser's language decides. This follows the
  same pattern as the duration format: the organization sets a default and each person may
  change it for themselves.
- **C-3 Invoice PDF.** Its language is configurable: the organization's language by default,
  changeable per client.
- **C-4 CSV and XLSX exports.** Headers are English by default, because spreadsheets and scripts
  depend on them. Exporting offers an option to choose another language.
- **C-5 Server error messages.** They are translated later, and each message gets a stable code
  (`reason`) for that. In the first increment they stay in English. The public API `/api/v1`
  stays in English.
- **C-6 Order with the glossary.** The catalog comes **before** PR-10 (the glossary of
  `DESIGN.md`), so that PR-10 only changes `horae.ftl`.
- **C-7 Check in CI.** A new rule of the Dioxus lint (`crates/dioxus-lint`) forbids loose visible
  text in `rsx!`. Every rule of that lint names a heading of the `dioxus-best-practices` skill,
  so this rule comes with its own section in the skill.
- **C-8 Formats.** Dates, money and durations keep coming from the single formatters of
  `horae-core` (PR-6), with English separators today. When other languages exist, a formatter
  receives the locale without changing its callers.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The interface reads from an English catalog, and nothing looks different (Priority: P1)

A developer opens any screen of Horae and finds no inline text in it: each label, heading,
button, placeholder, accessible name, toast and inline message comes from the English catalog. A
person using Horae sees exactly the same words as before, in the same places. When the developer
adds a screen, the build refuses inline text, so the catalog stays complete.

**Why this priority**: Every later increment depends on it. Without it, each language means
editing every screen, and new inline text keeps arriving.

**Independent Test**: For each migrated screen, compare the server-rendered HTML before and
after the migration in the screen's existing tests (with random identifiers and clock times
removed); it must be byte for byte the same. Then add an inline label to a screen and see the
lint fail.

**Acceptance Scenarios**:

1. **Given** a screen migrated to the catalog, **When** it renders in any state its tests cover
   (loaded, empty, error, read-only, editable), **Then** the HTML is identical to the HTML before
   the migration, and the screen's existing tests pass without changes.
1. **Given** the catalog, **When** a screen refers to a message that is not in it, or calls it
   without an argument the message uses, **Then** the server and WASM builds fail.
1. **Given** the lint, **When** a change adds visible text inside `rsx!` outside the catalog
   (a text node, a readable attribute or a prose prop), **Then** the check fails and names the
   file.
1. **Given** a message in the catalog that no code uses any more, **When** the checks run,
   **Then** they report it, so the catalog does not keep dead text.
1. **Given** a text with a number in it ("1 day", "3 days"), **When** it renders, **Then** the
   plural comes from the catalog message, not from a branch in the screen's code.

______________________________________________________________________

### User Story 2 - The glossary becomes a catalog change (Priority: P1)

The maintainer applies the glossary of `DESIGN.md` (PR-10): "People" for the workspace's people,
"Restore" instead of "Reactivate", "My settings" for `/settings`. The change edits the English
catalog and the tests that look for the old words, and no screen file.

**Why this priority**: It is the first visible payoff of the catalog, and the order is a recorded
decision (C-6). It also proves that wording lives in one place.

**Independent Test**: Apply PR-10 after the first increment and check that its diff touches only
the catalog and tests.

**Acceptance Scenarios**:

1. **Given** the English catalog, **When** a glossary word changes, **Then** every screen that
   shows that word shows the new one, and the screen files are unchanged.
1. **Given** a glossary word used in many messages ("People"), **When** it changes, **Then** it
   changes in one place in the catalog.

______________________________________________________________________

### User Story 3 - A person uses Horae in their language (Priority: P2)

A second language exists in Horae. An administrator sets it as the organization's default. A
member who prefers English sets English in My settings. Each of them sees every screen in their
language from the first paint after sign-in, with no flash of another language while the page
loads. Someone who has not signed in sees the sign-in page in their browser's language.

**Why this priority**: It is what a reader of another language gets from this feature, but it
needs User Story 1 first and a translated catalog to be visible.

**Independent Test**: With a test language added to the catalog, set the organization default
and a personal preference, then load screens as both people and as a signed-out visitor; check
the language of each page, its `lang` attribute, and that the server-rendered and hydrated pages
agree.

**Acceptance Scenarios**:

1. **Given** a person with a language preference, **When** they open any screen, **Then** it is in
   that language, whatever the organization's default and the browser's language are.
1. **Given** a person with no preference in an organization with a default language, **When**
   they open any screen, **Then** it is in the organization's language.
1. **Given** a person with no preference in an organization with no default, **When** they open
   any screen, **Then** it is in the first language of their browser's list that Horae has, or in
   English if it has none of them.
1. **Given** a signed-out visitor, **When** they open the sign-in page or see a sign-in notice,
   **Then** it is in their browser's language, or English.
1. **Given** a person who changes their preference, **When** they save it, **Then** the
   interface switches to the new language (a page reload is acceptable).

______________________________________________________________________

### User Story 4 - Error messages reach the screen in the reader's language (Priority: P3)

A member saves a time entry on a locked period. The message that explains the refusal is in their
interface language. A script that makes the same request over `/api/v1` gets the same refusal, in
English, with the same machine-readable code as today.

**Why this priority**: The server's messages are the largest block of text after the interface
(about 330 distinct messages in the operations), and C-5 schedules them after the first
increment.

**Independent Test**: Trigger a refusal from a screen in a test language and from `/api/v1`; the
screen shows the translated message and the API answer is unchanged.

**Acceptance Scenarios**:

1. **Given** an operation that refuses with a user-visible message, **When** the refusal reaches
   the interface, **Then** it carries a stable reason and the message's arguments, and the screen
   shows the catalog's message for that reason in the reader's language.
1. **Given** a refusal whose reason has no message in the reader's language, **When** it is
   shown, **Then** the English text appears.
1. **Given** any request to `/api/v1`, **When** it is refused, **Then** the response body is the
   same as before this feature, in English.

______________________________________________________________________

### User Story 5 - Documents without a browser use the right language (Priority: P3)

An organization invoices a client abroad. The administrator sets that client's invoice language;
the client's invoice PDFs come out in it, while other clients' invoices use the organization's
language. A project manager receives the budget warning email in their own language. An analyst
exports a report: by default the headers are the English ones their scripts expect, and they can
pick another language for a one-off export.

**Why this priority**: Emails, PDFs and exports have no browser to negotiate with, so each needs
its own rule. They come after the interface.

**Independent Test**: Set a client's invoice language and download one of its invoices; trigger a
budget email for a person with a preference; export a report with and without the language
option and compare the headers.

**Acceptance Scenarios**:

1. **Given** a client with no invoice language, **When** one of its invoices is rendered, **Then**
   its labels are in the organization's language.
1. **Given** a client with an invoice language, **When** one of its invoices is rendered, **Then**
   its labels are in that language, whoever downloads it.
1. **Given** a budget email, **When** it is prepared, **Then** it is in the recipient's preference,
   or the organization's language, or English.
1. **Given** an export with no language chosen, **When** it is produced, **Then** its headers and
   fixed values (`Yes`/`No`, `Active`/`Archived`) are byte for byte the English ones of today.
1. **Given** an export with a language chosen, **When** it is produced, **Then** its headers and
   fixed values are in that language, and the numbers keep the format exports use today.

______________________________________________________________________

### Edge Cases

- **A language that lacks a message.** The message appears in English; the rest of the screen
  stays in the reader's language. The build lists the missing messages as warnings, and a
  translation may ship incomplete.
- **A translation with a message or argument English does not have.** The build fails, so a
  translation cannot refer to text or values the code never passes.
- **Plurals.** Languages have different plural categories (English two, others up to six). Every
  text with a count is one message with the count as an argument, and each language writes its
  own forms. No screen builds "item(s)" or chooses between two messages by `n == 1`.
- **Sentences with a link or emphasis inside.** The whole sentence is one message, with the
  markup marked inside it, so a translator can move the link. No sentence is assembled from
  fragments, because word order differs between languages.
- **Server rendering and hydration.** The server and the browser render with the same language
  from the same compiled catalog, decided before the first paint. The page never shows English
  first and switches afterwards, and the server sets `<html lang>` to the language it rendered.
- **Regional browser languages.** A browser asking for `es-MX` gets `es` when Horae has `es` and
  not `es-MX`. A language Horae does not have is skipped in favour of the next one in the
  browser's list.
- **Emails and PDFs.** They have no browser: they use the recipient's or the client's language
  (User Story 5), never the language of whoever triggered them.
- **The component gallery** (`/components`, `pages/gallery.rs`) is a page for development with
  fictitious data. It is exempt from the catalog and from the lint.
- **Text that is not language.** The brand (Horae, a catalog term so a language can decline
  it), product names (Harvest, OpenAPI), currency codes, token prefixes (`horae_pat_`) and
  format patterns are not translated. The lint accepts them only as listed exceptions with a
  reason.
- **Accessible names.** `aria-label`, `title`, `alt` and `placeholder` are visible or audible text
  and go through the catalog like any label.
- **Tests that look for text.** In English the catalog reproduces the current text, so tests that
  match it keep passing. A wording change (PR-10) updates the catalog and the tests together.
- **Administration text.** The CLI, logs, panics and the demo seed stay in English: they are for
  operators and developers.

## Requirements *(mandatory)*

### Functional Requirements

#### Catalog (C-1)

- **FR-001**: All interface text MUST come from a catalog with one file set per language. English
  is the source catalog; every other language is a translation of it.
- **FR-002**: Code MUST reach each message through a generated method whose parameters are the
  message's variables. A missing message or argument MUST fail the build, on the server and in
  WASM.
- **FR-003**: The catalog crate MUST NOT depend on Dioxus or Axum, and MUST build for the server
  and for WASM, so the interface, the server (emails, PDFs, exports) and the API layer (sign-in
  page) share it. `horae-core` and `horae-types` MUST NOT depend on it.
- **FR-004**: Every language MUST be compiled into the binaries. Rendering a page MUST NOT fetch
  a catalog.
- **FR-005**: The build MUST validate each translation against English: syntax, duplicate ids,
  messages and variables English does not have (an error) and messages English has that the
  translation lacks (a warning; the English text is shown).
- **FR-006**: Messages that no code uses MUST be reported by the project's checks.
- **FR-007**: Message ids MUST follow documented conventions (screen, then what the text is;
  shared actions and common words under their own prefixes), and the glossary words of
  `DESIGN.md` MUST be catalog terms, so a language renames each one in one place.
- **FR-008**: Counts MUST use the catalog's plural forms, and sentences with markup MUST be single
  messages with the markup marked inside them (Edge Cases).

#### First increment: the interface to the English catalog

- **FR-010**: Every visible or audible text of the web app (`crates/horae/src`: pages,
  components, `app.rs`, `route.rs`) MUST come from the English catalog. This covers text nodes,
  readable attributes (`placeholder`, `title`, `aria-label`, `alt`, `label`), prose props of
  components, and the literals the pages show outside markup (toasts, inline messages, button
  states such as "Saving…").
- **FR-011**: The first increment MUST NOT change what a person sees or hears: the English catalog
  reproduces today's text exactly, including punctuation and the current wording that PR-10
  will change later.
- **FR-012**: The first increment MUST NOT translate or change server error messages, the sign-in
  page, emails, the invoice PDF, export headers or the CLI (C-5; they belong to later
  increments).
- **FR-013**: The first increment MUST be delivered as a series of changes that do not collide
  with the design PRs in progress (PR-4 screen states, the `Button` API, PR-8 buttons), following
  [Delivery](#delivery).
- **FR-014**: After the catalog infrastructure lands, any interface text that a change adds or
  moves MUST go into the catalog, including text added by design PRs.

#### Check in CI (C-7)

- **FR-020**: The Dioxus lint MUST have a rule that counts loose visible text in `rsx!`: text
  nodes with a letter, readable attributes and prose props of components, with literal values.
- **FR-021**: The rule MUST work as a ratchet like the existing rules: a per-file baseline taken
  when the rule lands, a failure when a file's count goes up, and a failure until the baseline is
  lowered when it goes down. At the end of the first increment every file's count MUST be zero,
  apart from the gallery and the listed exceptions, each with its reason.
- **FR-022**: The rule MUST name its own section of the `dioxus-best-practices` skill, which
  states the catalog rules of this specification for developers.
- **FR-023**: Literals shown from outside markup (toasts, messages built in Rust) cannot be
  recognised reliably by a lint. Their count MUST be reported on each change of the first
  increment as information; it MUST NOT fail the build.

#### Choosing the language (C-2)

- **FR-030**: Each person MUST be able to set a language preference in My settings, or clear it
  to follow the organization.
- **FR-031**: Administrators MUST be able to set the organization's default language in the
  workspace settings, or leave it unset.
- **FR-032**: The interface language MUST be resolved, in order, from the person's preference,
  the organization's default and the browser's `Accept-Language`, falling back to English. Before
  sign-in, only the browser's language and English apply.
- **FR-033**: The server MUST know the language before rendering, render the page in it, set
  `<html lang>`, and hand the same language to the browser so hydration does not change the
  text.
- **FR-034**: The sign-in page and its notices MUST come from the catalog, in the language
  resolved before sign-in.
- **FR-035**: The language picker MUST list only languages the catalog has, each named in its own
  language.

#### Server messages (C-5)

- **FR-040**: In the first increment, server messages MUST stay as they are, in English.
- **FR-041**: In their own increment, each user-visible message of the operations and the server
  functions MUST carry a stable reason and its arguments to the interface, which shows the
  catalog's message for that reason in the reader's language and falls back to the English text.
  Errors of `horae-core` are translated by their variant; their English text stays for logs and
  the API.
- **FR-042**: `/api/v1` responses MUST stay in English and MUST NOT vary with `Accept-Language`
  or the person's preference. The Harvest-compatible API follows the same rule.

#### Documents (C-3, C-4)

- **FR-050**: The invoice PDF MUST use the client's invoice language when the client has one, and
  the organization's language otherwise (English when neither is set). Who downloads the invoice
  MUST NOT change its language.
- **FR-051**: Administrators MUST be able to set or clear a client's invoice language.
- **FR-052**: Exports MUST write English headers and fixed values by default, byte for byte as
  today. The export screens MUST offer a choice of language, and an export in another language
  MUST translate the headers and fixed values only.
- **FR-053**: Emails MUST use the recipient's resolved language: their preference, then the
  organization's default, then English.

#### Formats (C-8)

- **FR-060**: Dates, money and durations MUST be formatted by the single formatters of
  `horae-core`, never by a screen. Screens and documents pass the value and, once languages
  exist, the locale; adding the locale MUST NOT change the formatters' callers.
- **FR-061**: Until a second language ships, formatted values MUST stay exactly as they are
  today (English separators, the app's date format).

### Key Entities

- **Catalog**: the set of messages and terms for one language; English is the source.
- **Message**: one translatable text with an id, its variables and, where it has a count, plural
  forms.
- **Term**: a reusable word inside messages, such as the brand and the glossary words.
- **Language preference**: optional, per person.
- **Organization default language**: optional, per organization; set by administrators.
- **Client invoice language**: optional, per client; set by administrators.
- **Export language**: a choice made for one export, English when not chosen.
- **Error reason**: a stable code per user-visible server message, with its arguments.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: At the end of the first increment, the lint counts zero loose visible texts in the
  interface's markup, outside the gallery and the listed exceptions. The study counted 1,416 at
  `be859203`; the baseline is retaken when the rule lands.
- **SC-002**: For every migrated screen, its server-rendered HTML in each tested state is byte for
  byte the same as before the migration (random identifiers and clock times removed), and its
  existing tests pass unchanged.
- **SC-003**: PR-10, applied after the first increment, changes only catalog files and tests.
- **SC-004**: Adding a language changes catalog files and the list of languages, and no screen
  file (checked with a test language in the second increment).
- **SC-005**: The first increment adds no more than 60 KB brotli to the served WASM bundle over
  master: the 42 KB of C-1 plus the English catalog's text (about 15 KB brotli in the study's
  estimate).
- **SC-006**: With a second language set, no page shows text in another language during load or
  after hydration, in the server-rendered pages the test suite covers.
- **SC-007**: An export made without choosing a language is byte for byte the same as before this
  feature.
- **SC-008**: Downloading the same invoice twice, by two people with different interface languages,
  gives the same labels.
- **SC-009**: No `/api/v1` response changes as a result of this feature (the `api-compat` and
  `api-v1` checks pass unchanged).

## Delivery

Each increment is planned and merged on its own. Only the first is ready to plan now.

### Increment 1: the interface to the English catalog (User Stories 1 and 2)

The work is mechanical but wide: about 2,000 texts in 91 files. The proof of concept moved 92
texts in three files in about an hour, so the study estimates 5 to 7 days for the screens and 1 to
2 for the infrastructure, the check and review. It ships as a short series of small changes
(200 to 300 texts each) so each rebases quickly.

1. **Infrastructure, no screens.** The catalog crate and its English catalog with the id
   conventions, the provider in `App`, the lint rule in ratchet mode with today's counts as its
   baseline, its section in the `dioxus-best-practices` skill, the unused-message check, and the
   crate in `nix flake check` (tests and clippy). From here on FR-014 applies. Its timing relative
   to the design PRs in progress is Question 1 below.
1. **The proof of concept's screens.** Settings, API tokens and Tasks, taken from `study/i18n`
   (`26e562bf`), with their before-and-after HTML comparison.
1. **Shared components** (`crates/horae/src/components`, about 145 texts): sidebar, layout,
   toast, modal, `DatePicker` day names, permission descriptions. Files that an open design
   branch changes (`button.rs`, the screen-state components of PR-4) wait for that branch to
   merge.
1. **Screens by area**, one change each, starting with those no open design branch touches:
   Clients, Invoices, Importers, Admin and permissions, Reports, New project, Approvals; Projects
   and Timesheet last. Before starting an area, the change lists the files that open design
   branches modify (`git diff --name-only master...<branch>`) and leaves those for after their
   merge.
1. **Close the ratchet.** Every file at zero apart from listed exceptions; SC-001 holds.

PR-10 follows (SC-003). PR-6's formatters (C-8) can land before, during or after this increment;
the screens keep calling them as they do.

### Later increments, in this order

1. **Server messages** with stable reasons (User Story 4, FR-041).
1. **Choosing the language**: preferences, the organization's default, `Accept-Language`, the
   sign-in page, `<html lang>`, and a test language to prove SC-004 and SC-006 (User Story 3).
1. **Documents**: budget email, invoice PDF with the client's language, the export language
   option (User Story 5).
1. **The first translation**, which is outside this specification.

## Open questions

These are the maintainer's to decide. Each carries a recommendation.

1. **When the infrastructure step lands relative to PR-4 and the `Button` API.**
   \[NEEDS CLARIFICATION: merge step 1 after PR-4 and the `Button` API, or before them?\] Once the
   lint is on, a design PR that adds or moves text has to use the catalog (FR-014), which would
   make the two branches in progress rework text they already wrote. **Recommendation:** merge
   step 1 after both of them; PR-8, not started, then uses the catalog from its first commit.
1. **Whether an issued invoice keeps its language.**
   \[NEEDS CLARIFICATION: when a client's invoice language changes, do invoices already issued keep
   the language they were issued in?\] C-3 says which language an invoice uses, but not whether a
   later change of the client's setting re-renders old invoices. **Recommendation:** an issued
   invoice keeps the language it was issued in, so the same invoice is always the same document;
   drafts follow the current setting.
1. **Who moves date formatting out of the screens.**
   \[NEEDS CLARIFICATION: does PR-6 also add the date formatter to `horae-core`, or does a step of
   this feature do it?\] About 15 places format dates with `chrono` patterns, and `DatePicker` has
   its own day names. C-8 puts the formatters in `horae-core` (PR-6), but PR-6's row in
   `design/IMPLEMENTATION-GAPS.md` names only durations. **Recommendation:** extend PR-6 to dates,
   with the same text as today; the day and month names it needs for other languages are decided
   when the second language arrives.

## Assumptions

- The first increment covers the web app only. The sign-in page is served by the API layer
  without Dioxus and moves to the catalog with language selection, because before sign-in it
  needs the browser's language anyway.
- Existing and new organizations start with no default language and people with no preference.
  While English is the only language, that changes nothing.
- Administrators set the organization's language and clients' invoice languages, with the same
  permission that governs the other workspace and client settings.
- A missing translation falls back to English at runtime, and the build warns. Whether CI should
  require complete translations is decided when the first translation ships.
- Changing the language preference may reload the page.
- Exports in another language translate headers and fixed values only; numbers keep the format
  exports use today (decimal hours, per DESIGN.md *Formats*).
- The duration-format preference of PR-6 does not exist in code yet. "Same pattern" refers to the
  rule in DESIGN.md *Formats*, and the two settings can share their screens when both exist.

## Out of scope

- The plan, tasks and code (`/speckit-plan` and `/speckit-tasks` follow the maintainer's review).
- Translating Horae into any language.
- Translating `/api/v1`, the Harvest-compatible API, the CLI, logs and the demo seed.
- Right-to-left layouts and non-Latin scripts beyond what the catalog itself supports.
- Locale-aware number, date and money formats (C-8 only prepares for them).
