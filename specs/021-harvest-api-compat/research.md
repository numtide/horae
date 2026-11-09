# Research: Harvest API v2 compatibility

Compiled 2026-11-09 for [spec.md](spec.md). Three parts: who could talk to a compatible Horae at
all (the ecosystem survey), what Harvest's public API v2 exposes, and what Horae serves today.
Sources are public documentation, package registries and open-source code read at the commit
linked. No Harvest account was signed in to and the Harvest API was not called. No downloaded
code was executed.

The Harvest documentation itself is summarised here, not copied: Horae rebuilds behaviour from
the public docs and does not reproduce their text (AGENTS.md, *Harvest parity*).

## 1. Ecosystem survey

### 1.1 The constraint that shapes everything

Every Harvest client sends its requests to `https://api.harvestapp.com/v2`, and every client that
signs in with OAuth2 uses `https://id.getharvest.com`. A compatible Horae can serve a client only
if one of these holds:

1. The client lets its user change the API host (and the identity host, for OAuth2).
1. The person can change one line of the client's code (a fork, a monkeypatch, a library option).
1. The person runs the client against a generic HTTP step they configure by hand.
1. The person intercepts the client's traffic with a locally trusted certificate authority.
   This works for self-run software, but it is an operational burden and is not a supported path.

### 1.2 Official Harvest software

| Client | Auth | Host configurable | Verdict |
|---|---|---|---|
| Harvest for Mac / Windows | Harvest ID sign-in | No; preferences have no server field ([Mac](https://support.getharvest.com/hc/en-us/articles/360048685351), [Windows](https://support.getharvest.com/hc/en-us/articles/360048180352)) | Unreachable |
| Harvest iOS / Android | Harvest ID | No ([Harvest ID](https://support.getharvest.com/hc/en-us/articles/360048180632)) | Unreachable |
| Chrome / Edge extension (MV3) | Harvest ID browser session | No; hosts compiled into the package's config. It also calls private, undocumented endpoints ([store listing](https://chromewebstore.google.com/detail/harvest-time-tracker/fbpiglieekigmkeebmeohkelfpjjlaia)) | Unreachable |
| Firefox extension (MV2) | Harvest ID browser session | Only a hidden developer override, and it points at Harvest's private iframe platform, not API v2 ([AMO](https://addons.mozilla.org/en-US/firefox/addon/harvest-time-tracker/)) | Unreachable; reproducing it would mean copying private endpoints, which AGENTS.md forbids |
| Harvest Button / Widget embeds (Asana, Trello, GitHub, Linear, Notion, …) | Harvest session on the platform host | No ([button](https://github.com/harvesthq/platform/blob/master/button.md), [widget](https://github.com/harvesthq/platform/blob/main/widget.md)) | Unreachable |
| Jira app | Hosted Atlassian app | No ([article](https://support.getharvest.com/hc/en-us/articles/360048180772)) | Unreachable |

Harvest's native integrations (QuickBooks Online, Xero, Stripe, PayPal, Slack, calendars, the
Asana import, Forecast) run between Harvest's servers and the partner. A compatible API does not
affect them; Horae would need its own integrations.

### 1.3 Automation platforms and SaaS connectors

| Platform | Connector auth | Host configurable | Generic HTTP fallback | Verdict |
|---|---|---|---|---|
| Zapier | OAuth2 via Harvest ID | No (closed) | Webhooks by Zapier | Generic HTTP only |
| Make | OAuth2 | No | HTTP module | Generic HTTP only |
| n8n (OSS) | PAT or OAuth2 (hidden fixed URLs) | No; host built in [GenericFunctions.ts#L29](https://github.com/n8n-io/n8n/blob/8fb9bbf7ef/packages/nodes-base/nodes/Harvest/GenericFunctions.ts#L29) | HTTP Request node | Generic HTTP, or a one-line fork |
| Pipedream (OSS components) | OAuth2 only | No; constants module | Code/HTTP step | Generic HTTP only (hosted) |
| Activepieces (OSS) | OAuth2 (`harvest:all`) | No; even its custom-call action is fixed ([index.ts](https://github.com/activepieces/activepieces/blob/main/packages/pieces/community/harvest/src/index.ts)) | HTTP piece | Generic HTTP, or a fork |
| Workato, Tray.ai, Power Automate, Integrately | Mixed (Tray uses a PAT; Power Automate's connector looks like API v1) | No | HTTP connectors | Generic HTTP only |
| Airbyte `source-harvest` (OSS) | PAT or OAuth2 | No config field; `url_base` in [manifest.yaml#L2632](https://github.com/airbytehq/airbyte/blob/d4f553943e/airbyte-integrations/connectors/source-harvest/manifest.yaml#L2632) | — | Self-hosted fork (one line, editable in Connector Builder) |
| Singer `tap-harvest` (OSS) | OAuth2 refresh only | No ([client.py#L50](https://github.com/singer-io/tap-harvest/blob/30b4ae7dfc/tap_harvest/client.py#L50)) | — | Fork plus identity endpoints |
| PostHog warehouse source | PAT | No, and hosted ([harvest.py#L22](https://github.com/PostHog/posthog/blob/7f4eb25af4/products/warehouse_sources/sources/harvest/harvest.py#L22)) | — | Unreachable |
| Fivetran, Runn, Productive, Databox | PAT or OAuth2 | No (closed SaaS) | — | Unreachable |
| Clockify | Imports a Harvest report CSV | n/a | — | Reachable through a CSV export, not the API |

### 1.4 Open-source clients and libraries

51 projects surveyed, 47 of them on API v2. Base-URL class: **CFG** user-configurable, **API**
configurable by the calling code, **dormant** an override exists but is not wired to user
configuration, **HC-1** hard-coded at one site, **HC-n** hard-coded at several.

| Project | Kind | Auth | Base URL | Evidence |
|---|---|---|---|---|
| [3AP-AG/harvest-client](https://github.com/3AP-AG/harvest-client) | Java library | PAT | CFG (`harvest.baseUrl`) | [reference.conf#L5](https://github.com/3AP-AG/harvest-client/blob/2a639cf05c/src/main/resources/reference.conf#L5) |
| [becoded/go-harvest](https://github.com/becoded/go-harvest) | Go library | PAT / OAuth token | API (`BaseURL` field) | [harvest.go#L23](https://github.com/becoded/go-harvest/blob/1ac13afbeb/harvest/harvest.go#L23) |
| [adlio/harvest](https://github.com/adlio/harvest) | Go library | PAT | API (`BaseURL` field) | [api.go#L20](https://github.com/adlio/harvest/blob/cfaf43f5c2/api.go#L20) |
| [gabor-boros/minutes](https://github.com/gabor-boros/minutes) | Rust crate + CLI | PAT | crate API (`with_base_url`), CLI HC-1 | [lib.rs#L100](https://github.com/gabor-boros/minutes/blob/af2fe28016/crates/minutes-harvest/src/lib.rs#L100) |
| [jolicode/harvest-php-api](https://github.com/jolicode/harvest-php-api) | PHP library (75k installs, archived 2026-09) | PAT | API (inject HTTP client) | [Client.php#L1667](https://github.com/jolicode/harvest-php-api/blob/05080a9205/generated/Client.php#L1667) |
| [wearerequired/harvest-api-php-client](https://github.com/wearerequired/harvest-api-php-client) | PHP library | PAT | API (plugin swap) | [Client.php#L57](https://github.com/wearerequired/harvest-api-php-client/blob/05137f24f8/src/Client.php#L57) |
| [spatie/laravel-harvest-sdk](https://github.com/spatie/laravel-harvest-sdk) | PHP library | PAT | API (subclass) | [Harvest.php#L16](https://github.com/spatie/laravel-harvest-sdk/blob/c6b14ad252/src/Harvest.php#L16) |
| [lionheart/python-harvest](https://github.com/lionheart/python-harvest) | Python library (archived, v1/v2 mix) | PAT / OAuth2 | API (constructor) | [harvest.py#L34](https://github.com/lionheart/python-harvest/blob/0934da448f/harvest/harvest.py#L34) |
| [simplyspoke/node-harvest](https://github.com/simplyspoke/node-harvest) | npm library (archived) | PAT | HC-1 | [client.ts#L27](https://github.com/simplyspoke/node-harvest/blob/07a921baa9/src/client.ts#L27) |
| [fastruby/harvesting](https://github.com/fastruby/harvesting) | Ruby gem | PAT | HC-1 (monkeypatchable constant) | [client.rb#L9](https://github.com/fastruby/harvesting/blob/968bf5ab68/lib/harvesting/client.rb#L9) |
| [VolodymyrBaydalka/Harvest.Api](https://github.com/VolodymyrBaydalka/Harvest.Api) | .NET library | OAuth2 + PAT | HC-1 | [HarvestClient.cs#L15](https://github.com/VolodymyrBaydalka/Harvest.Api/blob/f8b2e984d0/Harvest.Api/HarvestClient.cs#L15) |
| [kgajera/hrvst-cli](https://github.com/kgajera/hrvst-cli) | CLI | OAuth2 implicit (embedded client id), PAT by hand | HC-n, one choke point | [postman-request-command.ts#L179](https://github.com/kgajera/hrvst-cli/blob/0e70fbefda/src/utils/postman-request-command.ts#L179) |
| [dedene/harvest-cli](https://github.com/dedene/harvest-cli) | CLI | PAT + OAuth2 code | dormant (`NewClientWithBaseURL`) | [client.go#L55](https://github.com/dedene/harvest-cli/blob/bf9dcdf828/internal/api/client.go#L55) |
| [planetargon/harvest-tui](https://github.com/planetargon/harvest-tui) | TUI | PAT | dormant (`SetBaseURL`) | [client.go#L210](https://github.com/planetargon/harvest-tui/blob/d03f4fba31/internal/harvest/client.go#L210) |
| [jamesburns-rts/harvest-go-cli](https://github.com/jamesburns-rts/harvest-go-cli) | CLI on becoded | PAT | HC (library default) | [harvest.go#L111](https://github.com/jamesburns-rts/harvest-go-cli/blob/3a7744362c/internal/harvest/harvest.go#L111) |
| [evuez/ttyme](https://github.com/evuez/ttyme) | Haskell TUI | PAT | HC-1 | [Harvest.hs#L154](https://github.com/evuez/ttyme/blob/321fba3807/src/API/Harvest.hs#L154) |
| [raycast/extensions › harvest](https://github.com/raycast/extensions/tree/main/extensions/harvest) | Raycast (~2,400 installs) | PAT | HC-1 | [harvest.ts#L44](https://github.com/raycast/extensions/blob/c12374ce5a/extensions/harvest/src/services/harvest.ts#L44) |
| [andrejilderda/alfred-harvest](https://github.com/andrejilderda/alfred-harvest) | Alfred workflow | PAT | HC-n | [update-task.js#L8](https://github.com/andrejilderda/alfred-harvest/blob/66ae4ce666/src/update-task.js#L8) |
| [xendk/reaper](https://github.com/xendk/reaper) | Emacs | PAT | HC-1 (macro, redefinable) | [reaper.el#L622](https://github.com/xendk/reaper/blob/71c0fcb54a/reaper.el#L622) |
| [vigetlabs/yield-app](https://github.com/vigetlabs/yield-app) | macOS menu bar | OAuth2 code | HC (2 sites) | [HarvestService.swift#L8](https://github.com/vigetlabs/yield-app/blob/9fee1af17e/Yield/Services/HarvestService.swift#L8) |
| [jdmcleod/harvest_but_good](https://github.com/jdmcleod/harvest_but_good) | macOS | PAT | HC-1 | [HarvestAPI.swift#L37](https://github.com/jdmcleod/harvest_but_good/blob/ee6db5ec21/Sources/HarvestTimerCore/HarvestAPI.swift#L37) |
| [rainhead/sheaves](https://github.com/rainhead/sheaves) | Rust core + SwiftUI | PAT | HC-1 | [client.rs#L20](https://github.com/rainhead/sheaves/blob/f49409c87a/crates/sheaves-core/src/client.rs#L20) |
| [schpet/timer-reminder](https://github.com/schpet/timer-reminder), [harvest-vscode](https://github.com/yehoshuasandler/harvest-vscode) | VS Code | PAT | HC-n | [harvest.ts#L35](https://github.com/schpet/timer-reminder/blob/b1f1b35b47/src/harvest/harvest.ts#L35) |
| [taiste/harvest-mcp-server](https://github.com/taiste/harvest-mcp-server) | MCP server | PAT | HC-1 | [harvest-mcp-server.py#L45](https://github.com/taiste/harvest-mcp-server/blob/8246d7af76/harvest-mcp-server.py#L45) |
| [ianaleck/harvest-mcp-server](https://github.com/ianaleck/harvest-mcp-server) | MCP server | PAT | dormant (`HARVEST_API_BASE_URL` read, never passed) | [server.ts#L42](https://github.com/ianaleck/harvest-mcp-server/blob/0cbcd454df/src/server.ts#L42) |
| [southleft/harvest-mcp](https://github.com/southleft/harvest-mcp) | MCP server | OAuth2 code | HC (2 sites) | [config.ts#L61](https://github.com/southleft/harvest-mcp/blob/f9c1bf18d9/src/config.ts#L61) |
| [WebDevStudios/Hayloft](https://github.com/WebDevStudios/Hayloft) | Self-hosted Harvest archive | PAT | HC-1 | [harvest.ts#L4](https://github.com/WebDevStudios/Hayloft/blob/47099aaa14/src/lib/harvest.ts#L4) |
| [numtide/freelancer-toolbox › harvest](https://github.com/numtide/freelancer-toolbox/tree/main/packages/harvest) | Python scripts | PAT | HC-n | [`__init__.py#L22`](https://github.com/numtide/freelancer-toolbox/blob/c298b8826b/packages/harvest/src/harvest/__init__.py#L22) |
| [fs/harvest-notifier](https://github.com/fs/harvest-notifier) | Ruby | PAT | HC-1 (legacy `/api/v2`, `.json`) | [harvest.rb#L10](https://github.com/fs/harvest-notifier/blob/945f794d91/lib/harvest_notifier/harvest.rb#L10) |

About 20 more projects (other MCP servers, small CLIs, Swift/.NET/Go libraries) follow the same
pattern: a personal access token and a host constant.

**Counts over the 47 v2 projects.**

| Base URL | Count |
|---|---|
| User-configurable (end-user apps) | 0 |
| Configurable by calling code (libraries) | 8 |
| Override exists but unwired | 3 |
| One hard-coded site or one choke point | ~30 |
| Hard-coded at several sites | 6 |
| Hosted, cannot be repointed | 3 |

Auth: about two thirds use only a personal access token with `Harvest-Account-Id`; about one
third support OAuth2, and every one of those hard-codes `id.getharvest.com` and most embed a
client id registered with Harvest. Six call `GET id.getharvest.com/api/v2/accounts` to choose
an account.

### 1.5 Most-called endpoints

Counted over the 23 end-user apps (SDKs and ETL connectors call nearly every list endpoint and
are left out of the ranking).

| Endpoint | Apps |
|---|---|
| `GET /v2/time_entries` (by user, date range, `is_running`) and by id | 21 |
| `POST /v2/time_entries` (with `hours` logs time; without starts a timer) | 19 |
| `GET /v2/users/me` | 16 |
| `PATCH /v2/time_entries/{id}/stop` | 15 |
| `GET /v2/users/me/project_assignments` (embedded project, client, task assignments) | 14 |
| `PATCH /v2/time_entries/{id}` | 14 |
| `PATCH /v2/time_entries/{id}/restart` | 13 |
| `DELETE /v2/time_entries/{id}` | 11 |
| `GET /v2/company` | 8 |
| `GET /v2/projects`, `/v2/tasks`, `/v2/clients` | ~8 |
| `GET /v2/reports/*` (mostly `project_budget`, `time/*`) | 7 |
| invoices, estimates, expenses, contacts, roles | 5 (broad CLIs and MCP servers) |

The first nine rows cover the whole feature set of about 20 of the 23 apps; adding plain project,
task and client lists covers all of them. ETL and archive tools (Airbyte, Singer, Hayloft,
PostHog, n8n) need every list endpoint with `updated_since`, large pages and pagination links.

### 1.6 Behaviour clients rely on

| Assumption | Evidence | Consequence |
|---|---|---|
| Integer ids for every resource and for the account | Go `*int64`, Rust `i64`, Swift `Int`, zod `z.number().int()`, `%d` path formatting | Typed clients reject UUID strings. Needs a stable integer per record. |
| Many CLIs store project and task ids in their own configuration | hrvst-cli aliases, harvest-tui config | Keeping imported Harvest ids lets an existing configuration keep working. |
| Hours as decimal numbers, written as decimals | sheaves `hours: f64`; Alfred PATCHes `hours` | Convert at the boundary, exactly. |
| Timer semantics: `is_running`, `timer_started_at`, `stop`, `restart`, POST without `hours` starts a timer | 15 of 23 apps | Required for every timer UI. |
| Two pagination styles: page walkers read `next_page`/`total_pages`, link followers read `links.next` | n8n, Singer, Hayloft walk pages; Airbyte, PostHog, sheaves follow links | Return both, with absolute links on Horae's own host. Keep `next_page` filled even when serving cursors (sheaves documents that Harvest's null `next_page` mid-stream breaks page walkers). |
| `per_page` up to 2000 | sheaves asks for 2000 | Today Horae caps at 100. |
| `updated_since` for incremental sync | Airbyte, PostHog, Singer | Needs real update timestamps on every exposed resource. |
| `Harvest-Account-Id` in any letter case | hrvst and fastruby send `Harvest-Account-ID` | Header names are case-insensitive anyway; the value must parse as an integer. |
| No reliable `User-Agent` | Alfred sends none; node-harvest may send `undefined` | Harvest requires one; Horae should not. |
| 429 with `Retry-After`; Raycast reads `retry_after` from the JSON body | Raycast [harvest.ts#L59](https://github.com/raycast/extensions/blob/c12374ce5a/extensions/harvest/src/services/harvest.ts#L59) | Send both. |
| Parameters in the query string on POST/PATCH | Alfred | Accept query, form and JSON bodies. |
| Legacy `/api/v2/...` prefix and `.json` suffix | harvest-notifier, harvest-vscode, a Raycast fork | Cheap aliases. |

### 1.7 Conclusion on reach

- **As shipped, a compatible Horae reaches none of the Harvest ecosystem.** That covers
  Harvest's apps, its extensions, its marketplace and every connector. No end-user tool reads an
  API host from configuration.
- **What it does reach:**
  - Code people write themselves against the eight configurable libraries or the official samples.
  - Generic HTTP steps on automation platforms. The person rebuilds each call by hand.
  - Forks of open-source tools. About 40 of the 47 v2 projects need a one-line host change, and
    the personal-access-token ones (about two thirds) need nothing else.
  - OAuth-only tools additionally need Harvest-shaped identity endpoints on Horae and a patched
    client id.
- **"Works without problems" can honestly mean this:** a client that can be pointed at Horae (by
  configuration, a library option or a one-line change) and that uses a personal access token
  completes its normal workflows against Horae with no further changes. The workflows are
  signing in, listing projects and tasks, logging and editing time, timers, and incremental sync.
  The compatibility suite measures this per client. It cannot mean that Harvest's own apps,
  extensions or hosted connectors work.
- **The cheapest way to widen reach is outside this repository:** small upstream pull requests
  that add a base-URL option. Three tools already have the hook unwired (ianaleck MCP,
  dedene CLI, planetargon TUI); next are minutes, hrvst-cli, the Raycast and Alfred extensions,
  spatie's SDK and the Airbyte manifest, each a 1–10 line change. Whether to open them is the
  maintainer's call (outward-facing).

## 2. Harvest API v2 surface

Source: <https://help.getharvest.com/api-v2/> (each resource page under it). 138 documented
operations: 135 on the API host and 3 on the identity host.

### 2.1 Cross-cutting rules

- **Requests.**
  - Methods: GET, POST, PATCH and DELETE.
  - GET takes query parameters. POST and PATCH take a JSON or form body.
  - Required headers: `Authorization: Bearer <token>`, `Harvest-Account-Id: <integer>` and
    `User-Agent` (a missing one is answered with 400). The query-string form
    `?access_token=&account_id=` is also accepted.
  - Beta endpoints add the response header `Harvest-Api-Stability: beta`.
- **Types.**
  - Ids are non-negative integers; time-entry ids are documented as 64-bit.
  - Dates are `YYYY-MM-DD`; timestamps are ISO 8601 UTC with `Z`.
  - Times of day are `"14:59"` or `"2:59pm"`, depending on the company's clock setting.
  - Durations are decimal hours (`hours`, `rounded_hours`, `hours_without_timer`, report hours).
  - Money is a decimal in major units, with the currency named by a sibling `currency` field.
  - Percentages are decimals.
  - Weekly capacity is an integer number of seconds.
- **Status codes.**
  - 200; 201 with the new object.
  - 400 for a missing User-Agent and some malformed report parameters.
  - 401 when authentication fails; 403 when the record exists but the caller may not act on it.
  - 404; 422 for processing errors (for example `Invalid per_page parameter`).
  - 429 with `Retry-After` in seconds; 500.
  - **The error body format is not documented.** Observed shapes are `{"error", "error_description"}`
    for token failures and `{"message"}` for validation failures; both need confirming against a
    live account (see spec, Open Questions).
- **Rate limits.**
  - 100 requests per 15 seconds per account in general.
  - 100 requests per 15 minutes for `/v2/reports/*`.
  - Exceeding a limit returns 429 with `Retry-After`.
- **Pagination.**
  - The envelope holds `{<plural>: [...], per_page, total_pages, total_entries, next_page, previous_page, page, links: {first, next, previous, last}}`. Reports use `results` as the key.
  - `per_page` runs from 1 to 2000; 2000 is both the default and the maximum. A larger value is
    answered with 422.
  - 19 catalogue endpoints use cursor pagination, and the docs mark `page` deprecated on them:
    clients, contacts, tasks, projects, users, roles, user and task assignments, billable and cost
    rates, project assignments, teammates, tags, invoice and estimate messages, invoice payments,
    and item and expense categories.
  - On those endpoints `links.next` and `links.previous` carry an opaque `cursor`, while `first`
    and `last` stay page-based. If a request sends both `cursor` and `page`, the cursor wins.
  - Time entries, expenses, invoices, estimates and reports are page-based only.
- **Authentication.**
  - Personal access tokens.
  - OAuth2 authorization code (the token lasts 14 days and comes with a refresh token) and the
    implicit flow.
  - Scope names the accounts granted: `harvest:{ACCOUNT_ID}`, `harvest:all`, `all`.
  - `GET /api/v2/accounts` on the identity host lists the caller's accounts.
- **Company.** `GET /v2/company` returns:
  - Display settings: `week_start_day`, `time_format`, `date_format`, `clock`, `decimal_symbol`,
    `thousands_separator`, `currency_code_display`, `currency_symbol_display`.
  - Behaviour: `wants_timestamp_timers` (start/end tracking rather than durations) and
    `weekly_capacity`.
  - Feature flags: `expense_feature`, `invoice_feature`, `estimate_feature`, `approval_feature`,
    `team_feature`.
  - PATCH may change only `wants_timestamp_timers` and `weekly_capacity`.
- **Time entries.**
  - Create:
    - Duration mode: `hours` is optional; leaving it out starts a running timer.
    - Start/end mode: `started_time` defaults to now; leaving out `ended_time` starts a timer.
    - Both modes require `project_id`, `task_id` and `spent_date`; `user_id` defaults to the
      caller.
  - Update: `PATCH /time_entries/{id}`.
  - Timers: `PATCH .../restart` only works when the timer is stopped, `PATCH .../stop` only when
    it is running.
  - Delete: only when the entry is not closed and its project and task are not archived, except
    that administrators may delete closed entries.
  - `DELETE .../external_reference` removes the entry's link to an external item.
  - Filters: `user_id`, `client_id`, `project_id`, `task_id`, `external_reference_id`,
    `is_billed`, `is_running`, `approval_status`, `updated_since`, `from`, `to`.
  - Visibility: administrators see everything; managers see their own, their teammates' and
    their managed projects' entries; members see their own.

### 2.2 Endpoint inventory

| Resource | Ops | Operations |
|---|---:|---|
| Identity (identity host) | 3 | authorize (browser), token, accounts |
| Company | 6 | get, update; beta: reimbursement settings get/update, e-invoicing enable/disable |
| Clients | 5 | list, get, create, update, delete |
| Contacts | 5 | list, get, create, update, delete |
| Invoices | 8 | list, get, create (free-form or from tracked time and expenses), update (incl. line items), delete; beta: send e-invoice, e-invoice status |
| Invoice messages | 8 | list, new-message template, create and send, delete; mark sent / close / re-open / mark draft |
| Invoice payments | 3 | list, create, delete |
| Invoice item categories | 5 | CRUD + list |
| Estimates | 5 | CRUD + list |
| Estimate messages | 7 | list, create, delete; mark sent / accept / decline / re-open |
| Estimate item categories | 5 | CRUD + list |
| Expenses | 7 | CRUD + list; beta: approve, withdraw |
| Expense categories | 5 | CRUD + list |
| Tasks | 5 | CRUD + list |
| Time entries | 9 | list, get, create by duration, create by start/end, update, delete, delete external reference, restart, stop |
| Projects | 5 | CRUD + list |
| Tags (beta) | 1 | list |
| User assignments | 6 | list all, list per project, get, create, update, delete |
| Task assignments | 6 | list all, list per project, get, create, update, delete |
| Roles | 5 | CRUD + list |
| Users | 7 | list, me, get, create, update, archive, delete |
| Billable rates | 3 | list, get, create (per user, effective-dated) |
| Cost rates | 3 | list, get, create (per user, effective-dated) |
| Project assignments | 2 | per user, for me |
| Teammates | 2 | get, replace (per manager) |
| Time reports | 4 | clients, projects, tasks, team |
| Expense reports | 4 | clients, projects, categories, team |
| Uninvoiced report | 1 | uninvoiced |
| Project budget report | 1 | project budget |
| Detailed reports (beta) | 2 | time, expenses |

Reports require `from` and `to`, cannot span more than 365 days, and fall under the reports rate
limit. The documentation also has approvals, PTO, reimbursements, notifications, custom
permissions, saved reports and imports sections. The ecosystem barely uses them: only PostHog
reads PTO. They are out of scope here.

## 3. Horae today

### 3.1 What exists

- **Router.** `crates/horae/src/harvest/mod.rs`, mounted at `/harvest/v2`. It has 10 GET routes:
  `users/me`, `users`, `time_entries` list and get, `projects` list and get, `clients` list and
  get, `tasks` list and get.
- **Auth.** The web session cookie only (`harvest/auth.rs`). Bearer tokens and
  `Harvest-Account-Id` are ignored.
- **Ids.** UUID strings.
- **Units.** Hours and money are produced through floating-point (`minutes / 60`, cents / 100).
  Constitution 2.0.0 recorded this as an existing, unassessed deviation.
- **Pagination.** Relative links, page numbers only, `per_page` defaulting to 100 with a maximum
  of 100, and 400 for invalid windows.
- **Timestamps.** Projects, clients and users have no `updated_at`; tasks have no timestamps at
  all. So `updated_since` is approximate or ignored.
- **Contract.** Documented in `specs/001-time-tracking-invoicing/contracts/harvest-api.md`.
- **Consumers.** No client is certified. The e2e checks (`nix/checks/e2e*.nix`) and the
  pagination tests are the only callers in this repository.
- **The importer keeps Harvest's original ids.** `harvest_import_map (org_id, harvest_entity_type, harvest_id bigint, horae_id uuid)` records one row for each imported
  client, project, task, time entry and person (`person` added by 0053).
- **One function applies time-entry writes.** `commands::apply`, behind the
  `apply_timesheet_command` server function, handles creating, updating and deleting entries and
  starting and stopping timers for a given actor, with the same permission checks as the UI. Its
  command enum (`models/scoped_time.rs`) has no *restart* operation.
- **Settings.** `specs/harvest-parity.md` currently scopes "Existing integrations and API" to
  regression coverage only, and says not to add write endpoints. This feature changes that scope,
  and the parity tracker needs a decision row recording it.

### 3.2 Gap table

**Done** means the endpoint exists with Harvest-compatible behaviour apart from the cross-cutting
gaps (integer ids, token auth, pagination). **Partial** means the endpoint exists with known
field or filter gaps. **Missing** means Horae has the domain data but no endpoint. **No domain**
means Horae has no such concept yet, so the endpoint waits for a product feature.

Cross-cutting gaps that apply to every row: integer ids, token auth and account header, page size
2000 with cursors and absolute links, the error format, rate limiting, and exact decimal
conversion.

| Resource | Endpoint | Status | Notes |
|---|---|---|---|
| Identity | authorize, token, accounts | Missing | Needs Horae as an OAuth2 provider (spec US6) |
| Company | `GET /company` | Missing | Organization settings exist (name, currency, rounding, week start); map what exists |
| Company | `PATCH /company` | Missing | `wants_timestamp_timers`, `weekly_capacity` have no Horae setting |
| Company | reimbursement, e-invoicing (beta) | No domain | Out of scope |
| Clients | list, get | Partial | `updated_at` mirrors `created_at`; no `statement_key` |
| Clients | create, update, delete | Missing | Server functions exist (`create_client_profile`, `update_client_profile`) |
| Contacts | all 5 | No domain | Pending spec 012 (multi-contact billing identity) |
| Invoices | list, get | Missing | Invoices and line items exist |
| Invoices | create, update, delete | Missing | Domain exists; generation and edit go through requests and approvals |
| Invoice messages | all 8 | Partial domain | Status changes (sent, closed, draft) exist; email delivery does not |
| Invoice payments | all 3 | No domain | Only an invoice status of `paid`; no payment records |
| Invoice item categories | all 5 | No domain | |
| Estimates, estimate messages, estimate item categories | all 17 | No domain | No estimate concept (parity tracker: no spec) |
| Expenses, expense categories | all 12 | No domain | Pending spec 016 |
| Tasks | list, get | Partial | No timestamps; `updated_since` ignored |
| Tasks | create, update, delete | Missing | `create_task`, `update_task` server functions exist |
| Time entries | list, get | Partial | Missing filters `client_id`, `task_id`, `external_reference_id`, `is_billed`, `approval_status`; missing fields `user_assignment`, `task_assignment`, `external_reference`, `invoice`, `hours_without_timer`, `started_time`, `ended_time`; `approval_status` says `pending_approval` where Harvest says `submitted` |
| Time entries | create (duration), create (start/end), update, delete, stop | Missing | Domain operation exists (`commands::apply`) |
| Time entries | restart | Missing | No restart operation in the domain |
| Time entries | external reference delete | No domain | No external references stored |
| Projects | list, get | Partial | `updated_at` mirrors `created_at`; fee, notification and cost-budget fields absent |
| Projects | create, update, delete | Missing | Project creation and edit server functions exist |
| Tags (beta) | list | Missing | `project_tags` exist |
| User assignments | all 6 | Missing | `assignments` table |
| Task assignments | all 6 | Missing | `project_tasks`, `project_task_settings` |
| Roles | all 5 | No domain | Harvest roles are people groups; Horae has permission templates, which are a different concept |
| Users | list, me | Partial | Names split on first space; no `updated_at`; no capacity, timezone, roles fields |
| Users | get | Missing | |
| Users | create, update, archive, delete | Missing | `create_user` server function exists |
| Billable / cost rates | list, get, create | Partial domain | One current rate per user; no effective-dated history (parity: pending 013/014/015) |
| Project assignments | per user, me | Missing | Data exists; heavily used by end-user apps |
| Teammates | get, replace | Missing | `person_management_assignments` |
| Time reports | clients, projects, tasks, team | Missing | Report server functions exist |
| Expense reports | 4 | No domain | Waits for expenses |
| Uninvoiced report | 1 | Missing | Data exists (D-009 caveat on imported billed state) |
| Project budget report | 1 | Missing | Budgets exist |
| Detailed reports (beta) | time, expenses | Missing / no domain | Time only |

Of the 135 operations on the API host:

- **Partial: 10.** The existing reads: list and get for time entries, projects, clients and
  tasks, plus the user list and `me`.
- **No domain: 57.**
  - Estimates, with their messages and item categories.
  - Expenses, expense categories and expense reports.
  - Contacts, invoice payments, invoice item categories and roles.
  - The beta company endpoints, external references and detailed expense reports.
- **Missing with the domain in place: about 68.** This includes the invoice messages, whose
  status changes exist but whose email delivery does not.
- **Done: 0.** The cross-cutting gaps apply to every existing endpoint.

## 4. Planning decisions (Phase 0)

Each entry records the decision the plan follows, why, and what else was weighed. The
maintainer's answers to OQ-1 to OQ-3 and D-1 to D-9 are in spec.md *Clarifications*.

### R-1 Token format and storage

- **Decision**: a token is 32 random bytes from the operating system, encoded as base64url with
  a fixed `horae_pat_` prefix. Horae stores the SHA-256 digest of the whole string and its first
  eight characters for display. Lookup is by digest.
- **Rationale**: a 256-bit random secret needs no slow hash; SHA-256 of it cannot be reversed,
  which is what FR-002 and SC-006 ask. `sha2` is already in `Cargo.lock` (through
  `openidconnect`), so it becomes a direct dependency without a new download. The prefix lets
  secret scanners and people recognise the token. No surveyed client parses Harvest's token
  format; every one sends it as an opaque string.
- **Alternatives**: Argon2 (meant for low-entropy passwords, and slows every request); storing
  the token encrypted (a stolen database plus key would reveal it).

### R-2 Integer ids

- **Decision**: one table, `api_ids`, maps `(org_id, kind, number)` to a Horae UUID, with a
  `from_harvest` flag. Numbers are per organization and per kind. A migration backfills every
  existing record: first the rows in `harvest_import_map` with their Harvest id, then every other
  record in `created_at`, id order from `max(number) + 1`. Writes allocate in the same transaction
  as the insert, under a per-organization-and-kind advisory lock. The importer writes its Harvest
  id when it creates or matches a record; a collision with a native record gets the next number
  and a report line (OQ-2).
- **Rationale**: one table keeps the domain tables unchanged (no `bigint` column on every table)
  and keeps Principle III's UUID keys as the only primary identity. The import map already holds
  the Harvest ids, so the backfill is a query.
- **Alternatives**: a `bigint` identity column on each table (touches every table and every
  insert; can't keep Harvest ids without overriding identity values); hashing UUIDs into 53-bit
  integers (collisions; no Harvest id reuse).

### R-3 Account id

- **Decision**: `organizations.api_account_id bigint NOT NULL UNIQUE`, from a sequence, assigned
  in the same migration. Shown on the token page.
- **Rationale**: OQ-3 (a). A sequence never reuses a value.

### R-4 Exact decimals on the wire

- **Decision**: conversions live in `horae-core` (`api_decimal` module): minutes to hours text
  rounded half up to two decimals; hours text to minutes rounded half up, reusing
  `decimal::scale_decimal`; minor units to major-unit text at the currency's exponent; major-unit
  text to minor units, refusing extra digits. The server emits these as JSON numbers through
  `serde_json::value::RawValue` (`raw_value` feature) and reads them the same way, so no `f64`
  appears. Property tests check that every stored value round-trips (SC-005).
- **Rationale**: Principle I and the proposed clarification. `raw_value` writes a numeric
  literal verbatim without changing how the rest of the program parses JSON.
- **Alternatives**: `arbitrary_precision` (changes every `serde_json::Number` in the app);
  sending strings (breaks typed clients that expect numbers).

### R-5 Writes through the shared operation

- **Decision**: `server_fns::time_entries::commands::apply` becomes `pub(crate)` and is called by
  the Harvest handlers with the token's person and organization. Its `ServerFnError` codes map to
  Harvest statuses: validation 400 → 422, 403, 404, and conflicts stay 409 (the test account
  answers the daily cap with 409; see `harvest-parity.md`, 2026-11-05). The status for a locked
  or approved entry is not documented and is confirmed by recording (R-11). Restart becomes `TimesheetCommand::RestartTimer { entry_id }`. `StartTimer` and
  `RestartTimer` stop the person's running timer inside the same transaction (D-6), for the web
  app too. Catalogue writes call the existing server functions' inner operations the same way.
- **Rationale**: the amendment's "second transport, not a second path". Today's code already
  takes the actor and organization explicitly.
- **Blocked by**: the constitution amendment, approved on 2026-11-09 (constitution 5.0.0).

### R-6 Authentication extractor

- **Decision**: the Harvest extractor accepts, in order: `Authorization: Bearer`, the
  `access_token` query parameter, then the session cookie (reads only, D-7). With a token it
  checks `Harvest-Account-Id` (header or `account_id` query) against the organization's account
  id when present. `User-Agent` is not required (FR-008). Writes refuse a cookie-only request with
  401\.
- **Rationale**: matches what the surveyed clients send (research §1.6).

### R-7 Rate limits

- **Decision**: an in-memory fixed window per token in `AppState`: 100 requests per 15 seconds,
  and 100 per 15 minutes for report endpoints. A refusal is 429 with `Retry-After` and a JSON body
  carrying `retry_after` (the Raycast client reads it). Limits are per process.
- **Rationale**: Horae is deployed as one process; no new dependency or table. If it ever runs
  several replicas, the limit per process is still a bound.
- **Alternatives**: `tower_governor` (a dependency for a 30-line counter); a PostgreSQL counter (a
  write on every request).

### R-8 Pagination and links

- **Decision**: page-based lists keep their envelope with `per_page` 1–2000, default 2000 (D-4);
  the 19 catalogue lists also accept `cursor`, an opaque base64url encoding of the last
  `(updated_at, id)` seen, and fill `page`, `total_pages` and `total_entries` as well. Links are
  absolute, built from the request's `Host` and `X-Forwarded-Proto`, so they follow the tenant
  host once 020 lands.
- **Rationale**: research §2 (cursor docs contradict themselves; filling every field satisfies
  both readings).

### R-9 Update times

- **Decision**: add `created_at` and `updated_at` to `tasks`, and `updated_at` to `projects`,
  `clients` and `users`, maintained by the existing `set_updated_at()` trigger. Backfill
  `updated_at` from `created_at` (tasks from the import map's time or the migration time).
  `updated_since` filters on these columns.
- **Rationale**: FR-023. A trigger catches every write path, including the importer.
- **Caveat**: the migration snapshot tests compare whole rows; exclude the new columns there.

### R-10 Compatibility suite in Nix

- **Decision**: `nix/checks/harvest-compat.nix` starts PostgreSQL and the server as
  `nix/checks/tests.nix` does, seeds an organization, issues a token through the CLI test seed,
  and runs each client against it. Clients are fetched as fixed-output derivations, so the check
  itself has no network. The first clients: becoded/go-harvest (Go), minutes-harvest (Rust),
  hrvst-cli (Node), harvest-tui and dedene (patched base URL). Patches live under
  `nix/checks/harvest-compat/patches/` and are never sent upstream (D-8). PHP and Java libraries
  are added once their builds are confirmed offline.
- **Rationale**: FR-070 to FR-072. Go, Rust and Node have mature offline builders in nixpkgs.
- **Unconfirmed**: that each pinned client still builds with today's nixpkgs. The first task of
  step 1 checks this and drops a client that doesn't, recording why.

### R-11 Reference responses (D-9)

- **Decision**: a recording script, `scripts/harvest-record.sh`, sends only GET requests to the
  test (evidence) Harvest account, using `HARVEST_ACCESS_TOKEN` and `HARVEST_ACCOUNT_ID` from the
  environment. It refuses any other method, never writes those values to disk, and scrubs each
  response before writing it to `crates/horae/tests/fixtures/harvest_reference/`. Names, emails,
  notes, account and record ids and amounts become placeholders of the same JSON type, and
  numbers keep their count of decimals. It records one success response per implemented endpoint
  and the errors a GET can produce: 401, 403, 404 and 422 for an invalid `per_page`. It does not
  provoke 429.
- **Status**: blocked. No credentials for the test account were available to this session.
  Until they are, reference shapes are written by hand from the public docs (FR-073).

### R-12 Module layout

- **Decision**: convert `crates/horae/src/harvest/mod.rs` to `harvest.rs` plus `harvest/` in
  step 1, since this feature rewrites most of it. New files: `harvest/token_auth.rs`,
  `harvest/ids.rs`, `harvest/errors.rs`, `harvest/rate_limit.rs`, `harvest/time_entry_writes.rs`,
  `harvest/company.rs`, `harvest/assignments.rs`, `harvest/reports.rs`, `harvest/invoices.rs`.
