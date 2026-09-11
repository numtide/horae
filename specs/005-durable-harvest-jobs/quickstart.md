# Quickstart: Durable Harvest Import Jobs

1. Sign in as an organization administrator and open the Harvest importer.
1. Connect Harvest or select a CSV, choose dry-run or commit, and start the import.
1. Confirm the UI returns immediately with a queued job and then displays phase/progress.
1. Refresh or reopen the page; the same organization’s job history remains available.
1. Stop and restart the server during a running import; confirm the lease expires and the job resumes or reports a retryable failure.
1. Retry the job and confirm provenance/idempotency prevents duplicate records.
1. Cancel a running job and confirm committed data remains valid and no later batch starts.
