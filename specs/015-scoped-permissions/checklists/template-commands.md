# Local readiness: reusable-template commands

This checklist permits T053–T055, not full T037/T038 or activation.

- [x] FR-004/010/011/013/015/032 and C01 settle authority, preservation, naming and limit semantics.
- [x] Completed T035/T036 provide tenant-safe state and strict loaders.
- [x] `contracts/template-commands.md` defines typed intent, replay, revisions, atomic audit and exact create/delete effects.
- [x] Source-backed lock review covers current actor-before-organization writers and 0042 FK/trigger closure; the subset does not update their resource rows.
- [x] Follow-up adversarial contract review has no high/critical local finding; explicit isolation, permitted FK locks and future revision fencing are recorded.
- [x] T053–T055 map the acceptance cases to concrete tests, code/schema and verification paths; use existing crates/dependencies.
- [x] No public endpoint, operator entry point, legacy fallback, policy activation or real-data migration ships here; only disposable fixtures enable the policy.

Full requirements remain 12/16. Passing this checklist is readiness, not test
execution or full feature completion. Retain the complete integration backlog.
