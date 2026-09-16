# Acceptance evidence

Validated on 2026-09-16 on x86_64-linux, using the pinned Nix dev shell.

## Local checks

- `cargo clippy -p horae --features server --locked -- -D warnings` passed with
  `SQLX_OFFLINE=true`.
- `cargo check -p horae --features web --target wasm32-unknown-unknown --locked`
  passed.
- `dx build --platform web --fullstack true --force-sequential true --locked`
  produced matching server and WASM assets.
- `nix fmt -- --ci` and `git diff --check` passed.
- `projects-design.cjs`: eight checks passed, covering supported columns and
  native progress, resetting all three filters, 320/768/1440px short viewports,
  keyboard row-menu/edit access, CSV/XLSX export URLs, loading and failed spend
  (including retry), and empty-state action visibility for admin/manager/member.
  Role/read fixtures test presentation, not a replacement for server permission
  tests. Mutating API requests are blocked by this harness.
- `action-errors.cjs`: all four existing checks passed, including project status
  failures remaining visible while the form is opened and cancelled.

The initial design test failed against the previous bundle because the expected
New project action did not exist, before production changes were built.

## Isolation

The final browser checks used a newly created, demo-seeded database
`horae_projects_design_dev_20260916` and a loopback server on port 8092. The
temporary process had no Harvest configuration. The existing live service on
port 8080 and its imported business data were not changed. The error-regression
harness creates and toggles a disposable client only in the demo database.

## Review and boundaries

Reviewed the diff against the handoff, existing permission helpers, export
contract and utility framework. No new dependency, backend query, migration,
inline style or color literal was added. Existing project billing calculations
are unchanged. Unsupported scheduling/Delta placeholders were removed rather
than populated with invented data. Prototype-only bulk controls, pin/delete,
manager filtering and the inline-form-to-modal conversion remain deferred, as
listed in the specification.

This is a scoped list-page alignment, not pixel-perfect acceptance of every
prototype state. Prototypes were inspected as source, not rendered or captured.
Full Nix CI must pass before this delivery is merged.

## CI fixture correction

The first CI run exposed a compile failure in `detail_navigation`: its reduced
test crate includes the production Projects page but did not expose the icon
module or importer route added to that page. The application build and browser
checks alone did not compile this integration-test target.

The fixture now reuses the production icons and declares the importer route with
a minimal destination component. A new regression renders the empty Projects
list, checks both importer links and navigates to that destination and back.
The production UI and backend are unchanged by this correction.

- Reproduced the original missing-module/missing-route errors locally.
- `cargo test -p horae --features server --test detail_navigation --locked`:
  all five tests passed.
- `cargo clippy -p horae --features server --all-targets --locked -- -D warnings`:
  passed, including integration-test targets omitted from the earlier check.
- `cargo sqlx prepare --check --workspace -- --features server --all-targets`:
  passed against the isolated design-test database with `SQLX_OFFLINE=false`;
  the committed query cache is unchanged.
