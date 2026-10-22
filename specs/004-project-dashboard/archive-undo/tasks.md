# Tasks: undo project archive

Input: [spec.md](spec.md) and [plan.md](plan.md), bounded H102. Test fixtures are
disposable; no account writes, schema changes or policy activation.

## Setup and foundation

- [x] T001 Reconcile H102, references, existing authority and the H103 evidence
  limit in `specs/004-project-dashboard/archive-undo/spec.md` and `plan.md`.
- [x] T002 Inspect all shared Toast consumers and the design import before changes;
  record caller/regression scope in `specs/004-project-dashboard/archive-undo/plan.md`.

## US1 — Direct Undo (P1)

Independent acceptance: keyboard Undo has no extra dialog, restores active state
through reload and preserves project content. Dismiss does not write.

- [x] T003 [US1] Add missing-action, direct keyboard Undo, pending duplicate guard,
  dismissal and reload assertions in `crates/horae/tests/browser/project-detail-header.cjs`;
  observe the baseline failure before production changes.
- [x] T004 [US1] Add opt-in action rendering in `crates/horae/src/components/toast.rs`,
  preserving actionless markup and shared tokens.
- [x] T005 [US1] Connect archive-only Undo and direct authorized reactivation in
  `crates/horae/src/pages/projects.rs`, with transient typed state and focus recovery.

## US2 — Failure and authority recovery (P1)

Independent acceptance: retry/lost-response requests converge to active state;
denial cannot write or leave stale details/toasts; project navigation isolates state.

- [x] T006 [US2] Extend `crates/horae/tests/browser/project-detail-header.cjs` with
  retryable failure, lost response after commit, authority loss and project-navigation
  cases, preserving existing real-server and data-isolation assertions.
- [x] T007 [US2] Complete visible retry and access-loss handling in
  `crates/horae/src/pages/projects.rs`; no error may hide inside a closed modal.

## Cross-cutting delivery

- [ ] T008 Verify keyboard, themes, 320/390/768/1440, enlarged text and shared
  actionless notices; record actual artifact evidence and full Nix/WASM/format
  results in `specs/004-project-dashboard/archive-undo/plan.md`.
- [x] T009 Resolve two independent reviews and Spec Kit analysis; update H101/H102
  delivery and qualify the H103 conflict in `specs/004-project-dashboard/header-increment.md`.
- [ ] T010 Publish and protected-merge only with exact-head required CI green;
  record PR and integrated verification in `specs/004-project-dashboard/archive-undo/plan.md`.

Dependencies: T001 → T002 → T003 (red) → T004–005; T006 before T007; both stories
must pass before T008–010. Single author, no concurrent edits in this worktree.
Independent read-only review may accompany serialized coordinator tests. The
editor-reflow worktree is independent, but heavy builds/browser resources are not.

Traceability: UFR-001/002 and USC-001 → T003–005; UFR-003 and USC-002 → T003/006/007;
UFR-004 and USC-003 → T002/004/008–010; UFR-005 → T001/005/007–010.
