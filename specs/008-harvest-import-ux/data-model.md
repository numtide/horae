# Presentation Data Model

No new persisted entities or migrations.

## Connection

Reuse `ConnectionStatus` unchanged. Fetch state (loading/unavailable) is separate from loaded configuration, connection, expiry and binding. Failed fetch does not mean disconnected. Existing account/generation/revision snapshots and blocker logic retain confirmation semantics.

## Job

Reuse `JobStatus` kind/state/phase/counts/timestamps/report/error. Preserve all existing wire values. Add a read-only snake-case `retry_availability` enum:

| Value | Meaning |
|---|---|
| `unknown` | Missing metadata, unsupported version/kind or unrecognized shape; no retry affordance. |
| `available` | Known state/source prerequisites permit requesting retry. |
| `unavailable_state` | Not terminal failed/cancelled work. |
| `previous_account` | API job is from an earlier account generation. |
| `missing_upload` | CSV retry source is no longer retained. |

Default missing and future values to unknown during deserialization. Derive in bounded status/list queries from existing columns, current generation and upload existence, without N+1 requests. This snapshot is advisory; the actual retry remains transactionally validated. Keep `can_retry()` unchanged for existing consumers.

CSV ignores Harvest generation. A same-generation disconnected API job is not automatically generation-ineligible; reconnect guidance must not invent a server restriction.

## Report

- Known reports already contain source/mode at enqueue. Legacy absence gets “Mode unavailable”; never infer Commit from source kind.
- Only succeeded work has a completed-source result. Failed/cancelled work with a report stays partial even when retry is blocked. Failure without report cannot render success.
- Preserve exact per-entity outcome counts, inline/archive totals and complete downloads.
- Distinguish no selected report, no retained history and completed zero-record results. None proves lifetime absence of business data.

## Viewing context

Keep source, selection, current-preview origin, monitoring and dialogs separate from durable work:

```text
Ready → preview queued/running → preview result → explicit current confirmation → import result
History/source replacement/reload/account change → invalidate current confirmation as required
Status fetch failure → monitoring unavailable → resume the same job ID, never resubmit
```

Discard responses belonging to an old selection. Pending actions prevent double-clicks but do not replace server validation/idempotency. Account changes preserve history while invalidating the current API action context.
