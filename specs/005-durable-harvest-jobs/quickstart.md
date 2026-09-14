# Quickstart: Durable Harvest Import Jobs

This is the acceptance walkthrough for the complete feature. See
[adversarial-review.md](adversarial-review.md) for cases still under implementation,
including history restoration, checkpoints, and cancellation while running.

1. Sign in as an organization administrator and open the Harvest importer.
1. Connect Harvest or select a CSV, choose dry-run or commit, and start the import.
1. Confirm the UI returns immediately with a queued job and then displays phase/progress.
1. Refresh or reopen the page; the same organization’s job history remains available.
1. Stop and restart the server during a running import; confirm the lease expires and the job resumes or reports a retryable failure.
1. Retry the job and confirm provenance/idempotency prevents duplicate records.
1. Cancel a running job and confirm committed data remains valid and no later batch starts.

## Retry configuration

Set `HORAE_JOB_MAX_ATTEMPTS` in the server environment to an integer from 1 to
100 (default: 5). It includes the initial execution: 1 disables automatic retry.
Invalid or empty values reject startup instead of silently using a default.

Both API and CSV enqueue paths copy this policy into the job. Changing the
environment only affects new jobs; an idempotent enqueue or a manual retry keeps
the original limit. Manual retry resets the attempts consumed. Retryable errors
wait 2, 4, 8, … seconds, capped at 300 seconds. Leases remain five minutes, with
one worker per server instance.
