# Specification Quality Checklist: Harvest Data Importer

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-01
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

- **Re-scoped to API-first with provenance (this revision).** The primary source is a live Harvest REST API pull over OAuth2 (US1, FR-002/FR-022–FR-025); the Harvest CSV export is now a secondary, offline source adapter (US5). Both feed one source-agnostic engine, so the mapping, matching, dry-run, and resilience rules are shared.
- Matching is defined provenance-first in FR-012/FR-026: a persisted `(org, Harvest entity type, Harvest id) → Horae id` table (its own migration, additive — data-model.md) is looked up ahead of the composite natural key (client by name; project by code or client+name; task by name; time entry by the user/project/task/date/duration/notes composite). Provenance makes API matching exact and edit-robust and powers incremental re-sync; the composite key remains the fallback and the sole matcher for the id-less CSV source.
- The earlier "defer provenance" open decision is now **resolved as in-scope** — cheap and high-value once the API supplies stable Harvest ids (research.md §5). No [NEEDS CLARIFICATION] markers remain.
- OAuth2 credentials (access/refresh tokens + Harvest account id) are stored **encrypted at rest** in a new `harvest_credentials` table (its own migration), never surfaced to the browser or logs (FR-022). The OAuth callback is the one plain Axum route added (a browser redirect target), scoped to credential exchange (plan.md Constitution Check).
- Deferred to later versions: scheduled/automatic re-sync jobs, connecting more than one Harvest account per org, and importing Harvest entities beyond clients/projects/tasks/time entries.
