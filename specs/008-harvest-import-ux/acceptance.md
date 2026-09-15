# Acceptance Status: Harvest Import UX

## Current delivery

Specification/planning only, based on master `d32d56f`. No application, schema, credential or live-data change.

## Spec Kit execution

- `specify`: local template resolved, feature 008 created, quality checklist 16/16.
- `clarify`: prerequisite paths checked; zero questions needed. Scope, actors, entities/lifecycle, interaction/error/accessibility, quality bounds, integration failures, constraints, terminology and completion criteria are covered. Read-model details are resolved in planning.
- `plan`: setup script executed; research, model, UI contract and validation guide created. Pre/post constitution gates pass. Bounded read-only read-model investigation completed.
- `tasks`: setup script executed; 26 dependency-ordered implementation tasks created, all pending. Story counts: US1 4, US2 4, US3 6, US4 4; setup/foundation 3 and final verification/delivery 5.
- No extension hooks or preset overrides are installed. The agent-context update script is absent; technical context is recorded in the plan.
- Workflow uses checked-in `.claude/skills/speckit-*` instructions and `.specify/scripts/bash/`; no standalone `specify` executable is installed.

## Implementation gates — not executed

| Gate | Status |
|---|---|
| Read-model/UI implementation | Pending |
| Red/green presentation and interaction tests | Pending |
| Projection/authorization and CLI compatibility | Pending |
| Core/server tests, Clippy, WASM, SQLx, full Nix | Pending implementation |
| Browser layout, keyboard and zoom | Pending |
| Design comparison/deviation evidence | Pending |
| Real Harvest dry-run and data comparison | Pending implementation and separate operator go-ahead |

Record the tested commit, scenario/command, actual result and limitations for every executed gate. Never substitute another check's success for an unavailable check. Do not commit credentials, sessions or real data.
