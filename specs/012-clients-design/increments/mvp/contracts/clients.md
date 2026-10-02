# Clients UI and server contract

## Read boundaries

Active session required. Org comes from session, never form/route. Missing/foreign
client IDs share not-found; malformed IDs give safe validation errors.

- Keep `list_clients(include_inactive)` compatible with current pickers.
- Summary exposes catalog identity and authorized project progress only. Currency
  matches preferred OR visible project currency. Search is trimmed case-insensitive
  name substring. Lifecycle counts share the search/currency predicate.
- Detail includes inactive clients. Rates require manager authority and are absent
  from member payloads. Reuse project/invoice reads; never rely on UI hiding.
  Unknown fields show “Not set” or em dash, not zero.
- Key resources by client ID to avoid stale data under a new identity. A failed
  independent panel has error/retry, not an empty state.

## Save boundaries

Manager/admin checks on server/direct calls. Share validation and plugin events
with existing editors. Atomic profile save, explicit keep/replace/clear rate.
Currency changes with a saved rate require explicit replacement/clear and compare
edited original denomination/rate with locked current values. Legacy callers keep
their existing currency guard. Never rewrite historical rows or emit no-op events.

Shared form preserves errors/input, blocks duplicate pending submit and dismissal,
does not write on cancel, refreshes real data after success and restores focus.
Network-ambiguous creation is not automatically retried: explain uncertainty and
offer refreshed list inspection before explicit new save.

## Context navigation

- Optional client context enters existing project/invoice editors; bare routes
  remain valid and following a link creates no project/invoice.
- Existing project draft always wins, even if its client is empty/the same.
  Without a draft validate active context, prefill once, keep inherited currency
  unset. Normal editor autosave may persist a draft, not a project.
- Invoice RecoveryGate remains outermost. Pending exact payload/request ID wins
  over route context; preserve storage and lost-response recovery protections.
  Keep active picker and historical inactive-client billing semantics distinct.
- Invalid/foreign/ineligible context yields a notice/error, not first-row fallback.
- Existing rows link by ID; View in Projects retains client filter. Client detail
  highlights Clients; back/forward and dirty/pending navigation guards work.

## Pinned visual direction

**Context**: operational client management in existing Horae shell.

**Reference**: `design/project/app/12_Clients.dc.html` and
`12_Client Detail.dc.html`; no new visual identity/concept.

**Composition**: serif heading, compact actions/filters, rounded table panel.
Detail has back link, identity/actions, project panel and narrower billing/recent-
invoice column; stack on small screens.

**Style**: existing Invoicer fonts, warm surfaces/turquoise accents through
DESIGN.md tokens/utilities; no inline spacing/color hacks.

**Behavior**: real links, pending/error/empty states, shared labeled form and
keyboard focus. Inner table scrolling allowed; page horizontal overflow is not.

**Boundaries**: omit dev controls, contacts/bulk and deferred financial cards.
Keep their parent requirements; no mock totals or inert controls.
