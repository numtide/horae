# Invoice client identity regression

2026-10-21; branch `fix/invoice-client-labels`, base `b675d960`.

## Diagnosis and scope

Both invoice pages use `list_clients(false)` for labels, so deactivation removes
the name from their lookup. Preserve invoice authority and active-only creation
choices. No payment, archive-policy, permission-cutover or styling changes.
The earlier disposable Chrome audit reproduced the detail UUID; the list has
the same source defect. No Harvest mutation is required to establish this bug.

## Spec Kit

Reused US4 and MVP-003/007/011/012; added a history-label acceptance scenario and
T026–T030 rather than restarting the feature. `setup-tasks.sh --json` resolved
the existing MVP artifacts. Readiness checklist: 10/10 complete. No extension
hooks file. Parent feature 012 and global permission gates remain unchanged.

## Analysis and regression

`speckit-analyze`, limited to the authorized follow-up: MVP-003 maps to T027/028;
MVP-007 to T026/028; MVP-011 to T026/027/029; MVP-012 to T029/030. Four applicable
requirements, five tasks, 100% task coverage; no unmapped tasks, ambiguity,
duplication or critical/high findings in this slice. Constitution check passes:
read-only projection, unchanged money/minutes, PostgreSQL, typed server reads,
Nix verification. No extension hooks. Other feature/gate completion is not claimed.

RED: the focused `invoice-client-labels` suite through `run-design-checks.sh`
failed on the unmodified package from master `109402b9` (affected source identical
at b675d960): expected the saved name, received the UUID in the invoice list.
The runner created and stopped its own cluster at /tmp/horae-browser.X6cp2m;
no existing database used. Package: 43yrgwr7773qfqprb2smxv9ylf74501y-horae-0.1.0.

Implementation uses invoice-specific list/detail DTOs with the saved client
name and a batched identity-only query inside the existing manager snapshot.
Write response contracts and active-only picker remain unchanged; no CSS edit.
SQLx preparation passed using the Nix sqlx-prepare derivation with `--check`
removed, a disposable build-sandbox database and output-only cache export.
Two new descriptors were copied back; existing descriptors were preserved.
Offline tests and the standard prepare check still need to validate this cache.

## Independent review and corrections

Two independent static reviews completed before their conclusions were shared.
Both found the unchanged HTTP authorization test still expected the former flat
list response. Updated that assertion and added name assertions to both HTTP
read responses. Both also flagged the then-pending SQLx descriptors, now generated.
The first review requested multi-client coverage: the regression now tests two
clients in one organization, with a separate foreign organization excluded.

Simplified runner registration and reused the existing actor/tenant/revocation
matrix instead of duplicating it. Added a rendered detail assertion proving no
client catalog request is needed. Retained fail-closed missing-name handling:
the client FK alone does not guarantee same-organization linkage, so a corrupt
link must never disclose a foreign name. No schema repair is in scope.
Unsigned commits are expressly authorized by this goal; no global signing
configuration will be changed. Reviews were static only, not test-pass evidence.

Both correction reviews approved the static diff, with no remaining critical,
high or medium findings, conditional on executable validation. Optional corrupt
cross-org-link coverage is not claimed; current actor/tenant and revocation
tests remain the authorization regression gate. Neither review ran tests.

Post-correction Spec Kit analysis: the four follow-up requirements still map to
T026–T030, with no uncovered requirement, unmapped task, ambiguity, duplication
or constitutional conflict. Checklist remains 10/10; no extension hooks exist.
The helper's local feature pointer was restored, not included in the change.

## Executable validation

Initial Nix pass: formatting, Clippy (core/server all targets), standard SQLx
prepare check and release SSR/WASM package passed. Core: 191 passed. Server unit
suite: 1493 passed, 11 existing manual probes ignored, one new regression failed.
Its fixture compared the generation response's application timestamp with the
database-generated timestamp. Corrected the baseline to reload the persisted
invoice before lifecycle changes; all exact-value assertions remain. No runtime
change was needed. Full server and browser checks must finish on the final head.

Windows Chrome 155 via the existing Playwright connection passed active and
inactive list/detail/direct reload and creation-picker checks. Synthetic paid
invoice `LABEL-REGRESSION`, client `Historical <client> & agency`, disposable
cluster `/tmp/horae-invoice-labels.uOJW6O`; active option count 1, inactive 0.
Screenshots inspected in root `.scratch/invoice-labels-{active,inactive}-{list,detail}.png`.
No console errors; warnings came from a Chrome extension. Other browser tabs and
real Harvest records were untouched. Package: `hfylk1hy7wflz33pzabxf4c2sxpsijgq`.

Two independent final static reviews accepted the persisted-fixture correction
without loosening the assertions; no critical/high/medium findings remain.
The Chrome-only preview and its database were stopped, and only the owned test
tab was closed. Disposable evidence remains on disk; no existing data was deleted.

Next: finish the corrected full server and browser runs. Publish one draft PR to
start Nixbot in parallel, then mark ready only after local and required
published-head checks pass (`nixbot/nix-eval`, `nixbot/nix-build`). No merge.

Draft published: [PR #296](https://github.com/numtide/horae/pull/296),
commit `681d9459`. The default complete Nix browser suite passed, including
`invoice-client-labels`, on the tested release package. Only the test fixture
changed afterward; production code and browser scenarios are unchanged.
Second-pass Clippy/SQLx/format checks passed; full server execution is in progress.
SSH agent diagnostics stalled, so the owned push was cancelled and the same
branch was published through authenticated HTTPS without changing Git settings.

Final local server run passed: core 191; server unit 1494 passed, 11 pre-existing
manual tests ignored; all 15 integration binaries passed. This includes the
invoice-label regression, the rendered no-catalog detail test and the existing
actor/tenant/revocation matrix. Nix check outputs:

- tests: `g382cy9rr6h26bvidzpinx9mqzqzww52-horae-tests-0.1.0`
- clippy: `fmsqqp6x1akq5iw7g7iayyl72d42lrk4-horae-clippy-0.1.0`
- SQLx: `2y8n9x2rvsp6mn5gzhylycl4mzayafl3-horae-sqlx-prepare-0.1.0`
- browser: `dd2iswk0kxcgikm3qgw972cfsfpci1rl-horae-browser-checks`

T029 is complete. T030 remains open: PR #296 exists, but its required Nixbot
checks have not appeared yet. Investigate the CI trigger; do not infer a pass
from the absence of checks or replace CI evidence with local results.
