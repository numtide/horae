# Legacy report and invoice reader authority

Started at `60f60f9`, then rebased and verified against `5faed76` on
`feat/scoped-permissions`, dependency of draft #212. This increment addresses the
remaining legacy materialized-reader revocation work in T014/T039; it is not a newly
introduced regression or canonical policy activation.

## Scope and plan

1. Extract the production detailed-report, invoice-list and invoice-detail
   readers without changing their queries. Demonstrate revoked-actor disclosure
   with disposable PostgreSQL fixtures before adding authority fences.
1. Reuse `server_fns::snapshot::manager`: trusted organization/actor IDs,
   organization then actor SHARE locks, explicit REPEATABLE READ / READ WRITE,
   bounded prelude retries and existing timeout restoration. Materialize and
   commit before returning. Read invoice metadata and lines in that same snapshot.
1. Exercise current identity, tenant isolation, both race orders, coherent
   payloads, inherited settings and cancellation; extend the real-session HTTP
   harness for these endpoints. Run SQLx preparation, focused/full server tests,
   offline Clippy, WASM and formatting in an isolated target and database.

The READ COMMITTED project-export/CSV contracts remain unchanged: these three
readers follow the existing manager-snapshot contract. Preserve legacy
Manager/Admin authorization, filters, ordering, exact values, missing-invoice
404 and unrestricted accepted result sizes. Do not infer mixed-project invoice
policy, activate policy 1, alter UI, or write business data from these readers.

## Historical verification on the source branch

RED: `legacy_readers_reject_revoked_session_identity` failed with payloads from
all three production readers after both demotion and deactivation (six cases).
The test used the extracted existing queries before adding transaction fences.

RED: `legacy_invoice_detail_keeps_metadata_and_lines_in_one_snapshot` reproduced
an invoice header with 1,200 cents alongside concurrently updated 2,400-cent
lines. Both reproductions used the same pre-fix test binary and disposable data.

GREEN: all seven `server_fns::snapshot::reader_tests` pass, covering current
identity, tenant/resource filtering, gated/direct writer-first revocation,
reader-first retention, invoice coherence, cancellation with a one-connection
pool and inherited READ ONLY/isolation settings.

The HTTP harness's substring route lookup matched both `get_invoice` and
`get_invoice_editor`; it now accepts only the function name followed by Dioxus's
numeric route hash. The real-session matrix passes for all three endpoints,
including missing sessions, forged identity fields, foreign resources and both
demotion and inactivity committed after the initial session lookup.

`cargo test -p horae --features server --quiet` after rebasing: 1,110 in-crate tests
and 221 external tests pass; zero failures, 11 existing ignored tests. All commands run
inside the Nix dev shell against a dedicated PostgreSQL 17 cluster using a
private UNIX socket. The target is isolated, with debug symbols and incremental
compilation disabled and two build jobs.

Complete `cargo sqlx prepare --workspace -- --features server --all-targets`
passes after cleaning only this target's Horae package artifacts. The cache has
1,473 descriptors: all 1,465 existing entries plus eight new test queries, with
no deletions. After rebasing, offline all-target server Clippy and WASM Clippy
pass with `-D warnings`, with `DATABASE_URL` unset. Repository formatting passes.

No real database or Harvest account is used. Full `nix flake check` and
deployment/browser acceptance remain the parent feature's pre-merge gates;
this branch does not claim policy activation
or complete permission parity.

## Independent extraction after the Clients MVP

The replacement delivery is based on master `02f7b58`, the merge of #216.
It preserves that delivery's optional client filter and stable invoice ordering,
wrapping the current shared query rather than copying the older list query.
The snapshot helper's organization SHARE query is inlined unchanged so the
repair does not depend on unrelated project-writer changes. Original branches
and evidence remain preserved; the separation ledger is maintained in #218.

The new branch's seven focused reader regressions passed. Its full workspace
server/core suite then passed 1,127 tests, with zero failures and 11 pre-existing
manual scale/stress cases ignored. This includes the real-session HTTP matrix,
all seven reader regressions and the 35 Clients tests delivered by #216.

Full SQLx preparation with `--workspace -- --features server --all-targets`
passed against a fresh private PostgreSQL cluster: all 1,003 base descriptors
were retained unchanged and 18 were added for the extracted helper/tests.
Offline all-target server Clippy and WASM Clippy passed with warnings denied
and performance lints, with `DATABASE_URL` unset. `nix fmt -- --ci` passed.
Required new-head Flake Check remains pending. Historical source-branch gates
above are not evidence that this extraction has passed that gate. No production
data or policy changed.
