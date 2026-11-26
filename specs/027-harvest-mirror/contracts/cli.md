# Contract: CLI additions

Spec 006's commands, options, output envelope, session file handling and exit codes stay as they
are. The new commands follow the same rules: they call the server over HTTP with the session file,
never open the database, accept `--json`, and print diagnostics to stderr.

```text
horae mirror status
horae mirror enable [--interval MINUTES]
horae mirror pause
horae mirror resume
horae mirror run [--full] [--wait [--timeout SECONDS]]
horae mirror interval MINUTES
horae mirror parity [--all-months] [--wait [--timeout SECONDS]]
horae mirror cutover [--override-unexplained COUNT] [--wait [--timeout SECONDS]]
horae jobs list [--before UUID] [--limit 1..100] [--kind KIND]
horae jobs findings UUID --output FILE [--force]
```

- `MINUTES` is an integer from 1 to 60.
- `run`, `parity` and `cutover` print the job id, as imports do, and with `--wait` behave like `horae jobs wait`.
- `jobs list --kind` filters by kind; without it the list includes every kind, and older CLIs that
  know only the import kinds print other kinds with their kind name.
- `jobs report UUID` on a parity job prints its `ParityRunSummary`; on a pass, the per-entity counts.
- `jobs findings UUID` takes a parity job id and writes the run's CSV
  ([parity.md](parity.md#csv-download)) with the same private-temporary-file rules as `jobs errors`.
  On another kind it exits with the usage error code.
- `mirror status --json` prints `MirrorStatus` ([mirror.md](mirror.md#mirrorstatus-cratestypessrcharvest_mirrorrs)).

Exit codes reuse spec 006's: 0 success; the existing codes for usage, authentication, not found,
conflict and server errors. A cutover refused for unexplained findings is a conflict.
