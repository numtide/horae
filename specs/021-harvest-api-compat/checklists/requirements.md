# Specification Quality Checklist: Harvest API v2 compatibility

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-11-09
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

- The feature *is* compatibility with an external HTTP API, so headers, status codes, field names
  and paths appear in the requirements. They are Harvest's public contract, not Horae's
  implementation choices, and are kept. Horae's own storage, code layout and libraries do not
  appear in the spec. The constitution amendment names one existing function only to show which
  path the rule protects.
- The three open questions (OQ-1 OAuth2 provider, OQ-2 integer numbering, OQ-3 account id) and
  the nine lower-impact decisions (D-1 to D-9) were decided by the maintainer on 2026-11-09 and
  are recorded under *Clarifications*.
- The constitution amendment needs the maintainer's approval before delivery step 3 (writes).
  Unit conversion (steps 1–3) relies on the Principle I clarification.
- `harvest-parity.md` row D-020 records the approved scope change and the GET-only
  reference-account authorization.
