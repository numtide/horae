# Quickstart: Durable Harvest Import Jobs

1. Sign in as an organization administrator and open the Harvest importer.
2. Connect Harvest or select a CSV, choose dry-run or commit, and start the import.
3. Confirm the UI returns immediately with a queued job and then displays phase/progress.
4. Refresh or reopen the page; the same organization’s job history remains available.
5. Stop and restart the server during a running import; confirm the lease expires and the job resumes or reports a retryable failure.
6. Retry the job and confirm provenance/idempotency prevents duplicate records.
7. Cancel a running job and confirm committed data remains valid and no later batch starts.
