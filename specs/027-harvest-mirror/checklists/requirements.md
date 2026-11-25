# Specification Quality Checklist: Harvest mirror mode, parity checks and the durable jobs crate

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-11-25
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [ ] No [NEEDS CLARIFICATION] markers remain
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

- Four markers remain on purpose (OQ-1 to OQ-4): the maintainer asked for them to be recorded with
  a recommendation and left open. That is one more than the template's limit of three. Each one
  names its recommendation, and the plan proceeds on the recommendations; only OQ-1 changes
  requirements (FR-020 to FR-023) if answered no.
- The spec names Harvest's `updated_since`, the crate boundary and the API surfaces. Like the
  earlier Harvest specs (004, 005, 025), the feature is an integration and an internal redesign
  whose observable behaviour is defined by those names; the requirements still state outcomes, not
  code.
- FR-001 to FR-010 (the jobs crate) are an internal change whose user value is indirect: User Story
  1 (nothing changes) and User Story 9 (jobs run once, recover and retry) state it.
