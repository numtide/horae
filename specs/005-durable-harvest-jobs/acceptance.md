# Durable import browser acceptance

Observed on 2026-09-14 with application code at `df491b9`, Chromium
152.0.7977.82 and the real Dioxus SSR/WASM app. A separate process-compose stack
ran PostgreSQL on 55439 and the app on 8084, with freshly migrated demo data.
No server-function responses were mocked. The browser signed in through the
development login and uploaded an actual file through the importer UI.

## Fixture

The CSV contains 1,000 distinct valid one-hour entries for `admin@example.com`
and 999 invalid-date rows, with one new client, project and task. Valid rows
have unique notes. A test-only PostgreSQL trigger blocks the 501st valid entry
on an advisory lock held by a separate test connection. This establishes an
actual batch boundary without depending on import speed.

## Observed results

1. Starting preview returns a queued UUID before completion. After the first
   500 records, the checkpoint reports 503 outcomes including its three parents.
1. Reloading the page restores monitoring of the same running job and its history
   entry. Cancellation displays `cancelling`; retry is unavailable while cleanup
   remains blocked by the test's SQL gate.
1. Releasing that gate allows cancellation acknowledgement. The partial report
   still contains exactly 503 confirmed outcomes and no domain entries persist.
   This is cooperative cancellation as required by FR-010, not a promise that
   cancelling a future instantly interrupts an arbitrary PostgreSQL statement.
1. Clicking **Retry import** resumes the same preview to success. It remains
   read-only and is identified as a historical preview that cannot be committed
   directly. Its download contains all 999 errors with distinct source locations.
1. A new preview of the same selected file exposes **Commit this import**.
   Confirming it produces exactly 1,000 stored entries and 60,000 integer minutes.
   The UI displays **Import complete with 999 errors**, the full error count,
   the inline subset label and **Download all errors**.
1. The committed report's downloaded JSON lines exactly equal the recovered
   preview's 999 error records. Reloading, selecting that committed job from
   history and downloading again produces the identical records.
1. No browser `pageerror` events occur. The completed screen and the pending
   cancellation screen were visually inspected.

The successful recovered-preview job was
`01a0a0ce-63c8-7513-a7f3-4ce20b627f77`; its separately confirmed commit was
`01a0a0ce-9363-7342-b3c0-ada47802220d`. These identify temporary acceptance data,
not production records. Local screenshots and JSON-line downloads are in
`/tmp/horae-browser-acceptance.7n6UzB/`.

This closes T021's live UI/download path together with the API overflow, HTTP
authorization, archive retention and memory-stress tests in the review artifact.
The following separate acceptance establishes actual server-process recovery.
T007's CSV preview overhead and latest-head CI remain open.

## Actual process termination and recovery

On 2026-09-14, both SIGTERM and SIGKILL pass against the same built application,
started directly without the Dioxus development proxy. Chromium uses the real
administrator session, importer page and HTTP enqueue endpoint. Each scenario
has 1,000 distinct one-hour CSV entries. A test-only advisory-lock trigger blocks
the 501st insertion; the test observes the actual lock waiter and checks that
500 entries and a checkpoint have committed before signaling the server PID.

For each signal, the old process disappears and exactly 500 entries remain.
The browser displays the resume-monitoring control. After releasing the test
gate, advancing only that job's recovery deadline and starting a new process,
the same job succeeds on attempt 2 with exactly 1,000 entries, 60,000 minutes
and 1,000 created outcomes. Reloading and selecting its history entry displays
the completed report. Reimporting the identical CSV creates zero entries and
skips all 1,000.

| Signal | Old PID | New PID | Recovered job |
| ------ | ------- | ------- | ------------- |
| SIGTERM | 211774 | 211945 | `01a0a0db-366d-72b2-af4b-d05f1981d5db` |
| SIGKILL | 211945 | 212149 | `01a0a0db-627f-76d0-b3d6-5f52312cb9d6` |

The local harness exits successfully; its output is retained in
`/tmp/horae-browser-acceptance.7n6UzB/restart-direct2.log`. Its trigger and gate
are removed and its server processes are joined. The earlier interrupted test's
500-entry job also recovers to 1,000 entries without deleting or resetting data.
The initial proxy-based harness did not reach the offline UI assertion while
the development proxy remained alive. The direct run exercises that assertion
successfully. A subsequent harness selector was narrowed to the report banner
because the completion toast contains the same text; no production code changed.

The checked-in `nix/checks/e2e.nix` independently passes both signal scenarios
under systemd in [CI run 34867024976](https://github.com/numtide/horae/actions/runs/34867024976)
at `6bf66d6`. Its log records the real VM execution, including SIGKILL, recovered
jobs and reimports; the script completes in 81.52 seconds. SIGTERM additionally
requires a successful systemd service result. Both Format and Flake Check pass
for that revision. Together with cancellation/retry coverage, this closes T016;
it does not transfer that CI result to later commits.
