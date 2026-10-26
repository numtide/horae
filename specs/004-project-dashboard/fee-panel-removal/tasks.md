# Tasks: remove the Project Detail fee balances panel

Input: [spec.md](spec.md) and [plan.md](plan.md), bounded #208 F034 (fee panel).
Disposable fixtures only; no account writes, schema changes or policy activation.

## Setup

- [x] P001 Reconcile F034, the handoff, #208 FR-010/FR-017/FR-020/SC-004 and the
  open G01/O1/015 gates in `specs/004-project-dashboard/fee-panel-removal/spec.md`.
- [x] P002 Inventory panel callers, wiring and tests; record the relocation table
  in `specs/004-project-dashboard/fee-panel-removal/plan.md`.

## US2 — Balances in invoice preparation (P1; tests first)

- [x] P003 [US2] Relocate panel balance, responsive, recovery, revoked-authority
  and monthly-occurrence assertions to "Fees to invoice" in
  `crates/horae/tests/browser/invoice-preparation.cjs`. Keep the Project Detail
  absence check.

## US1 — Remove the panel (P1)

- [x] P004 [US1] Update `crates/horae/tests/detail_navigation.rs` and
  `crates/horae/tests/detail_navigation/detail.rs`: drop the fee probe/mock and
  the fee-specific late-denial test, keeping absence assertions.
- [x] P005 [US1] Add a real 401 status-write denial case (diagnostic hidden,
  bound Retry) to `crates/horae/tests/browser/project-detail-header.cjs`.
- [x] P006 [US1] Delete `crates/horae/src/pages/projects/fee_balances.rs`; remove
  its mount, `generation` and the stale comment from `crates/horae/src/pages/projects.rs`.

## Delivery

- [x] P007 Run focused SSR, server/WASM lint, formatting and the three browser
  suites on disposable data; record exact results in `progress.md`.
- [x] P008 Run Spec Kit analysis and two independent reviews (coordinator).
- [ ] P009 Publish only after coordinator review; no merge from this worktree.

Dependencies: P001 → P002 → P003–P005 → P006 → P007 → P008 → P009.

Traceability: PFR-001/PSC-001 → P004/P006; PFR-002 → P002/P006; PFR-003 →
P005/P006/P007; PFR-004/PSC-002 → P003–P005; PFR-005 → P001/P006;
PSC-003 → P007–P008.
