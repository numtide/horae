# Validation: Harvest Account Switching

Use the Nix dev shell and disposable PostgreSQL databases. Do not use real credentials or run committing imports against the operator's local account during automated acceptance.

Deployment is coordinated, not a mixed-version rolling upgrade: stop/drain old Horae servers and workers, apply migration 0029, start the rebuilt server/web bundle, then use the updated CLI and reload old tabs. Older processes do not enforce connection generations or OAuth revisions and must not run alongside an enabled Change account flow. The local preview remains on feature 006 until a separate deployment step.

1. Connect fixture account A, run a failed preview, disconnect, inspect the bound account and choose Change account. Cancel first and verify preservation; confirm next and connect fixture B.
1. Compare all business rows and retained reports before/after. API jobs from A remain inspectable but retry is rejected; a fresh B preview succeeds.
1. Repeat with provenance, queued API work, running API work and active CSV work. Each blocks the change without side effects.
1. Use barriers to race confirm against enqueue/retry and OAuth completion. Only the operation valid for the serialized state succeeds. No lock spans an external OAuth exchange.
1. Start OAuth in another session, change/disconnect, then complete the old callback. Confirm rejection, including A→B→A and a server restart.
1. Replay accepted pre-switch CLI identities (normal and future-dated). Confirm rejection before retention and expiry rejection after 30 days. Test missing/stale generation compatibility.
1. Check administrator/member/manager/inactive/demoted/foreign/missing sessions through registered HTTP endpoints and keyboard/modal states in UI acceptance.

Run core/server tests, all-target Clippy with denied warnings, WASM compilation, `cargo sqlx prepare --workspace -- --features server --all-targets`, Nix formatting and full flake checks. Record real results in acceptance.md; no completion claim before gates pass.
