# Constitution amendment for 022 Public API `/api/v1`

**Status**: Approved by the maintainer on 2026-11-18. Shape chosen the same day: a general
clause for token-authenticated APIs, and the principle renamed. PR A applies the text below
verbatim to `.specify/memory/constitution.md`.

**Version change**: 6.0.0 → 7.0.0 (MAJOR: Principle IV is redefined and renamed). If the
multi-tenant amendment (spec 020, which plans its own 7.0.0) lands first, this becomes the next
MAJOR after it and the two texts are merged; 020 changes the two exceptions, this one the API
clause and the title.

## Why

Principle IV, since 5.0.0, lets exactly one surface besides the server functions call the shared
mutation operations: "the Harvest-compatible API". `/api/v1` writes time entries (spec FR-031)
from clients that cannot call server functions, which is the same situation the 5.0.0 amendment
answered. Spec FR-022 requires the clause to cover `/api/v1` with the same bounds before any
write ships.

Naming each API would need an amendment per surface, although the bounds (token-resolved actor,
hashed tokens, no writes outside the shared operations, no cookie-only mutation) are the same for
all of them. The clause therefore names the class: Horae's token-authenticated HTTP APIs. The
title "Mutations Through Server Functions" stopped describing the rule in 5.0.0, when the rule
became "through the shared operations"; 021's amendment left the rename to the maintainer, who
chose it now.

## Proposed text

### Principle IV: title

Current: `### IV. Mutations Through Server Functions`

Proposed: `### IV. Mutations Through Shared Operations`

### Principle IV: replace the "Harvest-compatible API" clause

Current:

> **Harvest-compatible API (a second transport, not a second path)**: the Harvest-compatible API
> MAY invoke the same mutation operations a server function invokes for the equivalent action,
> with the actor resolved from a personal access token or an OAuth2 access token that Horae
> issued, stored only as a hash, and bound to one person in one organization.

Proposed:

> **Token-authenticated APIs (another transport, not another path)**: Horae's HTTP APIs for
> programs, today the Harvest-compatible API and the public `/api/v1`, MAY invoke the same
> mutation operations a server function invokes for the equivalent action, with the actor
> resolved from a personal access token or an OAuth2 access token that Horae issued, stored only
> as a hash, and bound to one person in one organization.

The four bullets that follow stay as they are:

> - It MUST NOT write to the datastore except through those operations, and MUST NOT hold its own
>   copy of any validation, permission or state rule.
> - A token MUST NOT grant more than its owner could do in the web app at the moment of the
>   request.
> - A request authenticated only by a session cookie MUST NOT mutate.
> - Token issue and revocation are themselves mutations and go through server functions.

with "It" read as "Such an API". Proposed first bullet: "Such an API MUST NOT write to the
datastore except through those operations, and MUST NOT hold its own copy of any validation,
permission or state rule."

### Principle IV: rationale, replace the last two sentences

Current:

> The Harvest-compatible API serves clients that cannot call server functions. Letting it call
> the same operations, rather than its own SQL, keeps authorization and validation in one place,
> which is what this principle protects.

Proposed:

> The token-authenticated APIs serve clients that cannot call server functions. Letting them call
> the same operations, rather than their own SQL, keeps authorization and validation in one
> place, which is what this principle protects.

Nothing else in the constitution changes. Principle I needs no clarification: `/api/v1` carries
integer minutes and minor units as they are stored.

## Sync Impact Report (draft)

- Version change: 6.0.0 → 7.0.0.
- Bump rationale: MAJOR. Principle IV is renamed, and its API clause is redefined from one named
  surface to the class of Horae's token-authenticated HTTP APIs.
- Modified principles:
  - IV. Mutations Through Server Functions → IV. Mutations Through Shared Operations. The opening
    rule and the two exceptions are unchanged. The "Harvest-compatible API" clause becomes
    "Token-authenticated APIs" and names `/harvest/v2` and `/api/v1`; its bounds are unchanged.
    The rationale's last two sentences are generalised.
- Added sections: none. Removed sections: none.
- Templates checked:
  - `.specify/templates/plan-template.md`: Constitution Check takes its gates from this file (no
    change).
  - `.specify/templates/spec-template.md`, `tasks-template.md`: no principle-driven change.
  - `AGENTS.md`: the *Architecture* paragraph lists the API surfaces and calls the Harvest API
    read-only; it is updated with PRs B, D and H, not with this amendment.
  - `README.md`: describes the Harvest API as read-only with session authentication; updated with
    PRs D and I, not with this amendment.
- Follow-up: earlier plans cite Principle IV by its old title; they are historical records and
  are left as written.
