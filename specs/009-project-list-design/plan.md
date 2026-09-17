# Implementation plan

1. Reuse the existing ProjectList, Menu, Combobox, NavIcon and empty-state styles.
1. Align header actions, wrap filters, replace the empty message with actionable
   empty states and remove unsupported numeric columns.
1. Replace the inline-width budget bar with native progress styled by tokens.
   Keep data loading/failure distinct from a genuine zero-spend project.
1. Add browser coverage for permissions, empty states, filtering, progress,
   keyboard access and responsive geometry. Update the renamed action in the
   existing action-error regression harness.
1. Run server Clippy, WASM check, formatting and isolated browser checks; publish
   one review PR. CI's full Nix gate remains required before merge.
1. Refine visual fidelity with additive utilities, existing chips and shared
   content-sized table tracks. Allow Menu/Combobox trigger utilities without
   changing compact defaults; cover both defaults and opt-in classes in tests.
1. Compare computed shared styles and dimensions on other routes before/after;
   exercise responsive layouts, menus and navigation in the isolated preview.

## Constitution check

No schema, queries, authorization, domain arithmetic, dependencies or mutation
paths change. All existing actions continue through their server functions.
Validation uses the Nix toolchain. No principle exceptions are required.
