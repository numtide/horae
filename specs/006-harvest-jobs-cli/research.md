# Research: Harvest Jobs CLI

## Transport

**Decision**: Use the already available asynchronous reqwest re-export from Dioxus fullstack. Give CLI-consumed server functions explicit stable POST routes. Send JSON argument objects; success is raw serialized result data, not a Result wrapper.

**Rationale/evidence**: Pinned Dioxus 0.7.9 macro source executes server-feature function calls locally. Implicit routes hash build context; endpoint constants are private. `server_fns/importers/authorization_tests.rs` already demonstrates real JSON HTTP calls. A separate read-only review confirmed this behavior. Both existing ureq and reqwest are available; async reqwest simplifies streaming and cancelling pending network waits.

**Alternatives rejected**: Direct function calls are not remote; reconstructing hashes is not portable; another Axum mutation router duplicates the authorized boundary; a new HTTP dependency is unnecessary.

## Authentication

**Decision**: Explicit private JSON file with canonical `server_origin` and only the existing `id` cookie value. Reject unsafe file modes, URL credentials/query/fragment/path prefixes, non-loopback plaintext, redirects and implicit proxies. Dispatch before `AppConfig::from_env`.

**Rationale/evidence**: `auth::make_session_layer` uses tower-sessions' `id` cookie and existing login; `require_admin` rechecks active user/role/organization. Server-only configuration must not prevent network-only commands running.

**Alternatives/tradeoff**: Database authority violates Constitution IV. A new device/login/token flow is out of scope. Operators provision/renew sessions through existing web login; this is not permanent unattended authentication. Never scrape browser stores or print credential-bearing errors.

## Idempotency

**Decision**: Optional `X-Horae-Idempotency-Key` carrying UUIDv7, scoped to organization and source kind. Compare mode/scope and a server-computed SHA-256 of CSV bytes atomically on conflict. Retain digest in the versioned payload after upload cleanup. Reject keys older than 24 hours or more than five minutes ahead.

**Rationale/evidence**: Current enqueue functions return a conflicting job without comparing inputs. PostgreSQL built-in `sha256(bytea)` avoids an extension/dependency. A submission window shorter than 30-day job retention prevents old keys creating replacements after deletion.

**Alternatives rejected**: File length is not identity; raw-upload comparison stops working after cleanup; replacing uploads mutates accepted work. A separate idempotency datastore is unnecessary. Resubmission is not manual retry.

## Source behavior

**Decision**: Preserve commit default and Incremental scope, which existing `run_api_import` resolves to a full fetch without a watermark. Reject unreadable/oversized CSV locally, missing connection before new API acceptance, and invalid CSV headers using existing parser validation. Record-level errors remain completed partial success.

## Results and files

**Decision**: One versioned JSON envelope per command; progress and pre-submission request ID on stderr. Retain identifiers on timeout/interruption/uncertain submission. Require an explicit output file for complete errors; stream into a private same-directory temporary file and publish only after successful EOF. Refuse overwrite unless explicitly requested.

**Rationale**: Bounded normal JSON and streamed archives prevent unbounded memory. Partial downloads must not look complete. Interrupting observation must never send a cancellation request.
