# Quickstart: Durable Harvest Import Jobs

This is the acceptance walkthrough for the complete feature. See
[adversarial-review.md](adversarial-review.md) for cases still under implementation,
including durable checkpoints and cancellation at resumable batch boundaries.

1. Sign in as an organization administrator and open the Harvest importer.
1. Connect Harvest or select a CSV, choose dry-run or commit, and start the import.
1. Confirm the UI returns immediately with a queued job and then displays phase/progress.
1. Refresh or reopen the page; the same organization’s job history remains available.
1. Stop and restart the server during a running import; confirm the lease expires and the job resumes or reports a retryable failure.
1. Retry the job and confirm provenance/idempotency prevents duplicate records.
1. Cancel a running job and confirm committed data remains valid and no later batch starts.

## Import history and recovery

The importer restores an active job from its latest history page when opened.
Select any history entry to follow it or view its terminal report. Use **Older
imports** and **Newer imports** to page through retained jobs; **Refresh history**
returns to the latest page. Failed and cancelled jobs expose **Retry import**.

If a status request fails, the screen stops its loading indicator and offers
**Resume monitoring**. This only resumes status reads; it does not enqueue a
second import. Cancellation remains in progress until the worker acknowledges
cleanup, and request errors are shown rather than silently ignored.

Historical previews are read-only: start a new preview to commit. A preview
submitted in the current page keeps its original source for confirmation, even
if a different CSV is selected before the report arrives.

## CSV commit recovery

Use a CSV longer than 500 records to exercise durable batch commits. After a
batch commits, progress is visible even though the import is still running.
Interrupt the worker or request cancellation, then retry the same job. Previously
committed records and report counts must remain intact, including legitimate
identical entries and record errors. The next attempt resumes after the last
committed record. The in-progress batch must not commit under an expired or
replaced lease.

This recovery path currently applies to durable CSV commits only. API imports
and dry-runs still need their checkpoint integration; previews continue to write
no domain data, including when they exceed the batch size.

## Retry configuration

Set `HORAE_JOB_MAX_ATTEMPTS` in the server environment to an integer from 1 to
100 (default: 5). It includes the initial execution: 1 disables automatic retry.
Invalid or empty values reject startup instead of silently using a default.

Both API and CSV enqueue paths copy this policy into the job. Changing the
environment only affects new jobs; an idempotent enqueue or a manual retry keeps
the original limit. Manual retry resets the attempts consumed. Retryable errors
wait 2, 4, 8, … seconds, capped at 300 seconds. Leases remain five minutes, with
one worker per server instance.
