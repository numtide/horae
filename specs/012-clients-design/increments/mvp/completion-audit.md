# Clients MVP completion audit

Scope: all four journeys in the authorized Clients MVP increment, not the whole
parent feature 012. Production code reviewed at `eee87ba`, with browser-fixture
corrections verified in the full gate at `5de6a8b`. Subsequent delivery-document
changes must not be treated as new functional evidence. The published head and
its actual checks on [PR #216](https://github.com/numtide/horae/pull/216) are the
authority for CI, not this document's existence.

## Requirement evidence

Paths below are relative to the repository root. Database and browser mutation
evidence uses synthetic records in disposable databases.

| Requirement | Implementation and verification evidence | Result |
| --- | --- | --- |
| MVP-001: searchable/filterable list | `pages/clients.rs` and `pages/clients/filters.rs` compose name, state and preferred/visible-project currency. Filter unit tests reconcile counts; `clients-errors.cjs` exercises loading, failure/retry, injected empty versus real no-match, combined filters and clear. `clients-access.cjs` checks hidden-project counts/currencies. | Functional evidence passed. |
| MVP-002: real detail and work | `pages/clients/detail.rs` loads persisted identity/billing and real project/invoice rows. Client database tests reconcile invoice states/currencies and omit foreign records; `detail_navigation.rs` verifies real links, stale-response cancellation and unknown versus zero. `clients-access.cjs` verifies visible projects and an actual invoice link by role. | Functional evidence passed. |
| MVP-003: current access boundaries | `server_fns/clients.rs` derives session identity, scopes SQL by organization and active actor, filters before aggregation and omits member billing in serialization. Client/project privacy tests and browser direct-call replay cover anonymous/inactive/member/lead/manager/admin and foreign identifiers. No feature 015 activation. | Functional evidence passed. |
| MVP-004: shared create/edit | `pages/clients/form.rs` reuses common form and modal controls. `core/src/client.rs` owns trusted validation; server profile functions persist atomically. `clients.cjs` covers create/reload/edit/cancel, pending duplicate/dismissal protection and lost acknowledgements; `clients-errors.cjs` covers load/save failure and retained input. | Functional evidence passed. |
| MVP-005: financial history | Explicit keep/replace/clear rate intent and locked original currency/rate comparison reject reinterpretation and stale financial edits. Tests compare whole project/invoice rows, verify concurrent edits/demotion and distinguish unset/zero. New Project's browser regression verifies the shared client editor preserves its existing zero rate. | Functional evidence passed. |
| MVP-006: existing lifecycle/import | Existing single-client activation/deactivation remains non-cascading. `clients.cjs` snapshots linked rows across both transitions and follows Import to its real destination; `action-errors.cjs` retains mutation-error isolation. No contacts, new archive policy, deletion or bulk behavior. | Functional evidence passed. |
| MVP-007: contextual navigation | Route wrappers reuse Projects, New Project and invoice preparation. `client-context.cjs` compares complete existing drafts, including empty-client drafts, and project/invoice record snapshots. `invoice-preparation.cjs` verifies exact pending payload/request recovery wins over different or invalid context. Route/DOM tests cover identity and navigation guards. | Functional evidence passed. |
| MVP-008: handoff/system fidelity | Clients and Client Detail handoffs govern composition. Existing components/tokens are reused with only client-scoped layout/readability rules. `visual-review.md`, independent `finish-review.md` and `design-system-review.md` record evidence, adaptations and limits. | Scoped design review passed. |
| MVP-009: keyboard and responsive behavior | `clients-visual.cjs` covers both themes, four widths, 100/200% text and short-screen editors; 64 confirmation images were individually inspected. Native modal trapping, Escape and focus return are asserted. Responsive and mobile navigation regressions cover the surrounding shell. | Scoped browser evidence passed. |
| MVP-010: Harvest comparison | `research.md` records dated official sources and read-only browser observations for fields, search, currency, permissions and lifecycle, plus explicit unverified behaviors and authorized differences. Existing valid evidence was reused; no server-validation or complete Harvest parity claim is made. | Comparison delivered with documented limits. |
| MVP-011: isolated verification | Browser runner creates its own PostgreSQL cluster; mutation suites guard the loopback test target/socket. Nix tests, SQLx prepare and deployment tests use disposable/sandboxed databases. The full local Nix gate passed on `5de6a8b`: 121 core tests, 817 server tests plus other targets, release server/WASM, Clippy, SQLx, all 21 browser suites and both deployment checks. | Full local gate passed. |
| MVP-012: scoped delivery | Isolated `feat/clients-mvp` worktree/branch, unsigned commits and PR #216 ready for review. Author-led adversarial and independent UI reviews are distinguished, with no unresolved critical/high findings. Full local checks and CI passed on implementation head `5de6a8b`. No merge/queue changes. Latest published-head checks remain authoritative for the final handoff. | Implementation delivery verified; recheck final head. |

`pages/`, `server_fns/` and `models/` above are under `crates/horae/src/`;
browser filenames are under `crates/horae/tests/browser/`.

## Success criteria and process

- MVP-SC-001/002: filter/domain/DB assertions and real-session browser payload
  checks prove the displayed scope and negative-access boundaries, not merely
  absence of hidden markup.
- MVP-SC-003: reload persistence, failed/cancelled-write checks, whole historical
  row comparisons and concurrent transaction tests establish the save contract.
- MVP-SC-004: draft and business-record snapshots plus exact pending invoice
  recovery tests establish navigation without business-record creation/loss.
- MVP-SC-005: the complete visual/keyboard matrix and focused cross-screen
  suites passed. The first full browser gate found obsolete regression selectors;
  subsequent runs exposed shared fixture assumptions. After those corrections,
  the complete default 21-suite browser gate passed on `5de6a8b`, including both
  permission matrices, navigation, errors/retry and all visual configurations.
- MVP-SC-006: implementation-head CI passed in run `36964454445`, and PR #216
  is ready for review. Final handoff must verify the latest published head after
  the delivery-document commit, not rely solely on this earlier green run.
- Actual Spec Kit reconciliation, clarification, planning, tasks, analysis and
  implementation are recorded in `progress.md`; all 10 increment readiness
  checklist items are checked. The post-implementation analysis maps all 18
  requirements/success criteria to 25 tasks. There are no extension hooks in
  this checkout. Parent requirements were retained, not marked complete.
- No migrations, dependency/lockfile changes, replacement design system, native
  app, new integration or unrelated refactor appears in the increment diff.
  Commit signature inspection confirms unsigned increment commits.

## Delivery evidence and final-head verification

T023 is complete: the full local Nix/format process finished with exit 0 and
`all checks passed!` on `5de6a8b`. Earlier browser failures were corrected and
all 21 suites passed together in their unchanged default order. No failed gate
was skipped. This verifies the current `x86_64-linux` system, not unsupported
cross-platform targets.

T025 implementation-delivery checks passed: [CI run 36964454445](https://github.com/numtide/horae/actions/runs/36964454445)
completed successfully for exact head `5de6a8b95e4191e7724a310d22ca6ca61411dacf`.
Flake Check took 48m36s and Format 56s. PR #216 had no submitted reviews/comments
pending and was made ready for review after these checks passed. No merge or
queue operation occurred.

This documentation closes the implementation record, not the checks for a
future commit. Before the final goal handoff, verify that the latest published
head (including this documentation-only update) has green checks and that the
worktree has no unpublished delivery changes. If not, keep the goal active.

## Limits and next increments

Harvest browser evidence is partial; server mutations there were intentionally
not exercised. The design detector was unavailable, not passing. Visual review
is not a whole-application accessibility audit, and independent UI review is not
an independent backend security audit. Contacts, expanded permission profiles,
bulk/export/destructive actions, changed archive policy, new billing terms and
financial summary/provenance panels remain parent-feature work.
