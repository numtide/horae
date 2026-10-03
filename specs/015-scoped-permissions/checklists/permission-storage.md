# Local readiness: non-activating permission storage

This checklist permits T035/T036 only, not runtime policy activation.

- [x] FR-032 settles trimmed, case-insensitive organization-local template names.
- [x] `contracts/permission-storage.md` defines schema, tenant references, strict loading and required tests.
- [x] Canonical grants, independent administrative identity and explicit provenance remain separate; presentation is not authority.
- [x] C01 template deletion preserves identity and grants; restrictive references prevent implicit cascading or rebinding.
- [x] Bounded adversarial design review corrected the identity/provenance coupling; no high/critical design finding remains in this storage contract.
- [x] T035/T036 define implementation/test paths and retain commands, audit/replay, 50-template limit, full matrix and activation in their owning tasks.
- [x] Only isolated disposable PostgreSQL fixtures may be migrated; no real data, legacy mapping, runtime consumer, dependency or UI change is included.

Readiness is not successful test execution. Record actual results in quickstart.md
before completing the tasks. The full requirements checklist remains 12/16.
