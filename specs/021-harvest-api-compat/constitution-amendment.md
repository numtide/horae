# Proposed constitution amendment for 021 Harvest API compatibility

**Status**: Proposed. Needs the maintainer's approval. `.specify/memory/constitution.md` is not
changed by this specification.

**Version change**: 4.0.0 → 5.0.0 (MAJOR: Principle IV is redefined). If 020's amendment lands
first, this becomes the next MAJOR after it, and the two texts are merged; they touch different
paragraphs of Principle IV (020 changes the two exceptions, this one the opening rule).

## Why

The constitution allows "additional non-mutating surfaces (e.g. a read-only compatibility API)",
which "MUST NOT become a second mutation path". Harvest clients write time through
`POST/PATCH/DELETE /v2/time_entries` with a bearer token. They cannot call Dioxus server
functions. So the API needs to be a mutation path.

The rule's purpose is stated in its rationale: one typed, authorized mutation path keeps
authorization and validation in one place. The amendment keeps that purpose. The single path
becomes the shared domain operation that server functions already call (for time entries,
`commands::apply`), not the server-function transport. The compatibility API may call the same
operation and nothing else.

Principle I bans floating point for durations and money. Its 2.0.0 follow-up already recorded that
the compatibility API converts through floating point, and left that unassessed. Harvest's wire
format is decimal hours and decimal major units. Converting integers to decimal text and back
needs no floating point, so Principle I itself does not change. The amendment adds a clarifying
paragraph so the wire format is not read as permission to compute in decimals.

## Proposed text

### Principle I: add after the XLSX exception (clarification)

> **Decimal wire formats (not an exception)**: an interface whose external contract requires
> decimal hours or decimal major currency units, such as the Harvest-compatible API, MAY write and
> read those decimals as text converted exactly from and to the stored integers, using integer
> arithmetic only. Hours are written rounded half up to two decimals and read back rounded half up
> to whole minutes. Amounts are written with the currency's own number of decimals and refused on
> input when they carry more. Totals are computed in integers before conversion; the decimal
> values are never summed, compared or stored. Floating point remains forbidden for these
> conversions.

### Principle IV: replace the opening paragraph

Current:

> All data mutations MUST go through Dioxus `#[server]` functions (session-authenticated,
> role-checked), except as the two narrow exceptions below. The UI MUST NOT issue ad-hoc
> client-side fetches for data reads or writes. Additional non-mutating surfaces (e.g. a read-only
> compatibility API, exports) are permitted but MUST NOT become a second mutation path.

Proposed:

> All data mutations MUST go through Horae's shared mutation operations: the authorized,
> validated domain operations that Dioxus `#[server]` functions call. Each operation takes the
> acting person and organization, checks their role and permissions, and applies every rule for
> that change. The two narrow exceptions below are the only writes outside them. The UI MUST NOT
> issue ad-hoc client-side fetches for data reads or writes; it calls server functions.
>
> **Harvest-compatible API (a second transport, not a second path)**: the Harvest-compatible API
> MAY invoke the same mutation operations a server function invokes for the equivalent action,
> with the actor resolved from a personal access token or an OAuth2 access token that Horae
> issued, stored only as a hash, and bound to one person in one organization.
>
> - It MUST NOT write to the datastore except through those operations, and MUST NOT hold its own
>   copy of any validation, permission or state rule.
> - A token MUST NOT grant more than its owner could do in the web app at the moment of the
>   request.
> - A request authenticated only by a session cookie MUST NOT mutate.
> - Token issue and revocation are themselves mutations and go through server functions.
>
> Other additional surfaces (exports, health, plugin hosts) are permitted but MUST NOT mutate.

Rationale paragraph: add one sentence at its end.

> The Harvest-compatible API serves clients that cannot call server functions. Letting it call
> the same operations, rather than its own SQL, keeps authorization and validation in one place,
> which is what this principle protects.

## Sync Impact Report (draft)

- Version change: 4.0.0 → 5.0.0.
- Modified principles:
  - **I. Exactness:** clarifying paragraph on decimal wire formats. The rule is unchanged; this
    alone would be a PATCH.
  - **IV. Mutations Through Server Functions:** the opening rule is redefined from "through
    `#[server]` functions" to "through the shared mutation operations those functions call". A
    bounded clause for the Harvest-compatible API is added. The two existing exceptions are
    unchanged. The title may become "Mutations Through Shared Operations"; that is left to the
    maintainer.
- Templates: plan template's Constitution Check takes its gates from the constitution (no change).
- AGENTS.md: the *Architecture* paragraph says the Harvest API is read-only and lists "two API
  surfaces". Update it with the implementation, not with this amendment.
- Contract `specs/001-time-tracking-invoicing/contracts/harvest-api.md` says the API is strictly
  read-only. It is superseded by 021's contract when step 3 of the delivery order lands.
- Follow-up: `harvest-parity.md` scopes "Existing integrations and API" to regression coverage
  with no write endpoints. It needs a decision row recording the maintainer's approval of this
  feature.
