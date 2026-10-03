# Legacy report conversion readiness

- [x] T042 inventories the concrete job-to-organization FK inversion.
- [x] The state contract specifies discovery, organization SHARE, tenant/job recheck and rediscovery.
- [x] Independent lock research checked worker archive/checkpoint, claim, cleanup and size-one pools.
- [x] Explicit READ COMMITTED preserves fresh candidate revalidation after waiting.
- [x] Actual writer races must preserve bounded reports, exact chunks and live leases.
- [x] T065–T067 separate failing tests, implementation and verification.
- [x] No new schema, dependency, user-facing policy or real-data operation is required.

This is local readiness only. Full T042, authorization integration and policy
activation remain open under the existing incremental implementation authority.
