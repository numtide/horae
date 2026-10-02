# Clients MVP adversarial review

Scope: the increment from `f5bacfbd` through `1393796` and the subsequent
bounded visual correction batch. This is the implementation author's code and
cross-feature review. The separate [finish review](finish-review.md) and
[design-system review](design-system-review.md) are independent UI reviews,
not independent backend security audits.

## Code and behavior examined

| Boundary challenged | Evidence and result |
| --- | --- |
| Cross-organization reads and hidden financial data | Client detail joins the active session actor's organization. Summaries admit only projects allowed by `project_read_access.can_view_progress` before counting or aggregating currencies. Member payloads omit billing rather than hiding its markup. Disposable DB and browser direct-call tests cover foreign IDs, missing IDs, inactive actors, manager/member/lead/freelancer combinations and restricted rates. |
| Invoice authority and denomination | Client invoice reads require manager authority and an authorized exact client, then reuse the existing organization/client-filtered invoice query. Tests reconcile draft/sent/paid/void records in four currencies and exclude a second client and organization. UI labels each amount with its own currency; there is no mixed-currency aggregate. |
| Stale routes and partial failures | Keyed detail resources prevent old identity and invoice panels surviving navigation. Virtual DOM tests exercise delayed/failed responses and request cancellation. Browser error tests prove actual retry paths; missing project totals remain unknown instead of becoming zero. |
| Trusted validation and concurrency | Core validation rejects empty/overlong/null-containing names, unsupported currencies and invalid, negative, fractional-cent or overflowing rates. Profile saves lock and recheck the active manager and client before comparing the financial snapshot. Concurrent demotion/edit tests wait for actual database lock dependencies. |
| Financial history and no-op events | Profile writes update client defaults only. Rate changes use explicit keep/replace/clear intent; existing-rate currency changes cannot silently relabel money. Tests compare complete historical project/invoice rows and client row versions. Unchanged saves do not produce the event-dispatch change signal. |
| Existing callers and lifecycle | The catalog DTO and picker signature are preserved. Project inline creation shares core validation while keeping its existing transaction and authority. Legacy updates retain the locked-row rate/currency guard. Activation/deactivation remains non-cascading; browser fixtures compare project/invoice rows before and after both transitions. |
| Ambiguous writes and pending forms | One shared form blocks duplicate submit, Escape/backdrop dismissal and navigation while pending. Failed input remains present. Lost-create-acknowledgement tests prove list inspection finds the committed record without an automatic duplicate or a cancel undo. |
| Context versus drafts/recovery | Project draft restoration wins over route context even for an empty saved client or malformed incoming ID. Context prefill happens only without a draft and retains inherited currency. Invoice RecoveryGate remains outermost; exact pending request/payload survives different and malformed client links. Tests compare complete saved drafts and business-record snapshots. |
| Shared styles and regression coverage | Changes are client-scoped and use incumbent components/tokens. No global table, form, modal or utility-generator defaults changed. All 64 confirmation captures were inspected; keyboard/geometry/contrast assertions and the eleven-route responsive suite passed. |
| Isolation and scope | Browser mutation suites validate a loopback test instance and a `/tmp/horae-browser.*` disposable PostgreSQL socket. No production data, real Harvest writes, mail, schema migration, dependency, contacts/bulk/delete behavior or partial feature 015 cutover is introduced. |

The relevant production modules, route/navigation changes, browser assertions,
core tests, client database tests and project privacy regression were inspected.
Existing test assertions were retained when resource names and form selectors
changed. No new critical/high issue was found in this final code pass.

## Findings corrected during implementation

- Invoice client-catalog failure lacked a retry action. The existing preparation
  flow now has accessible loading/error/retry without changing RecoveryGate;
  the failure/retry browser suite verifies recovery.
- The first visual matrix exposed search wrapping, compressed identity columns,
  detail title/count order, oversized billing at enlarged text and low-contrast
  small labels. One scoped batch corrected these; new browser assertions first
  reproduced the failures, then passed on the rebuilt artifact.
- A browser filter check interacted before its reloaded list was ready. It now
  waits for the real fixture row and asserts the entered query and filtered
  output, with no sleep, blanket retry or removed requirement.
- The full browser gate exposed a stale mobile Clients resource wait. A related
  New Project regression still targeted the replaced client editor. Both now
  exercise the actual summary resource/shared modal while preserving their
  navigation, currency rejection, input retention and persistence assertions.
  Both complete suites passed in a focused disposable rerun. The corrected
  full gate remains required; no production code changed in this follow-up.

## Spec Kit post-implementation analysis

### Full-suite fixture follow-up

The next full browser run exposed shared-seed assumptions: project editing
renames ACME-01 and fee preparation reassigns TECH-01 to Acme. Error/visual
readiness checks now use the stable code rather than the changed name.
Context navigation owns two synthetic project fixtures, asserts both are
visible before filtering, and verifies each client's inclusion/exclusion by
exact ID. Full row snapshots still prove navigation causes no business writes.
No assertion was removed, suite skipped, production filter changed or fixture
reset to disguise the preceding tests' valid writes. The complete sequential
browser and Nix gates remain required for closure.

The full sequential rerun passed context navigation and found that the access
suite mutated a fixed seed administrator rather than the administrator actually
selected by dev login after another suite added an admin. It now resolves the
session actor through the real observed `get_me` endpoint and applies/restores
fixture role changes to that identity. Authorization assertions are unchanged;
there is no application permission change or test-only grant.

### Artifact analysis

Ran `check-prerequisites.sh --json --require-tasks --include-tasks` with
`SPECIFY_FEATURE_DIRECTORY=specs/012-clients-design/increments/mvp`, then reviewed
the specification, plan, contracts, tasks and constitution. The helper's feature
pointer side effect was restored to its pre-call value; no specification
remediation was necessary.

| Requirement | Covering tasks |
| --- | --- |
| MVP-001 | T006–T009 |
| MVP-002 | T010–T012 |
| MVP-003 | T002, T006–T007, T010–T011, T021 |
| MVP-004 | T013–T015, T021 |
| MVP-005 | T013–T014 |
| MVP-006 | T009, T013, T016 |
| MVP-007 | T002, T017–T020 |
| MVP-008 | T002, T009, T012, T015, T022 |
| MVP-009 | T015, T022 |
| MVP-010 | T001, T025 |
| MVP-011 | T004–T005, T021–T023 |
| MVP-012 | T003, T023–T025 |
| MVP-SC-001 | T008, T021 |
| MVP-SC-002 | T010, T021 |
| MVP-SC-003 | T013, T021 |
| MVP-SC-004 | T017, T021 |
| MVP-SC-005 | T022 |
| MVP-SC-006 | T025 |

Metrics: 18 requirements/success criteria, 25 tasks, 100% task coverage, no
unmapped tasks, ambiguity, duplication or constitutional conflict found.
Coverage is not completion: full Nix/cache checks and PR/CI delivery remain
separate gates. The design detector was unavailable and is not counted as a pass.

Closure: T023 passed the full local gate and T025's implementation head `5de6a8b`
passed CI run `36964454445`. All 21 browser suites passed together, including the
fixture-isolation corrections above. PR #216 is ready for review, not merged.
The [completion audit](completion-audit.md) maps all requirements to evidence
and requires checking the final published head after the documentation update.
Parent feature 012 remains incomplete.
