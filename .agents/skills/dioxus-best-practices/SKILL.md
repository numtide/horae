---
name: dioxus-best-practices
description: Use when writing, reviewing, splitting or optimising Dioxus 0.7 components, pages, hooks or data loading in Horae (crates/horae/src/components, pages, app.rs), or when diagnosing extra re-renders, unstable props, refetching or stale use_resource results, loading flashes, lost local state after a reload, SSR/hydration gaps, or full-page reloads from links.
---

# Dioxus 0.7 best practices

## Overview

Horae pins Dioxus **0.7.9**. Every rule below was read from that tag's source
(`github.com/DioxusLabs/dioxus`, tag `v0.7.9`; paths are relative to `packages/`).
The evidence for each rule lives in `references/`. Several common beliefs about
Dioxus are wrong for this version, so go by these rules, not by memory or by
third-party skills.

Core principle: **a component re-renders when a signal it read changes, or when its
parent re-renders and its props compare unequal.** Most performance bugs come from
props that can never compare equal, or from reads that subscribe the wrong scope.

## Props: what compares equal (`core-macro/src/component.rs`, `props/mod.rs` `memoize_impl`)

| Prop shape | Parent re-render ⇒ child re-renders? | Do instead |
|---|---|---|
| `on_x: move \|v\| …` inline closure | **Yes, every time.** It is a new `Callback` with `ptr_eq` equality, and its box is kept until the parent unmounts | `let on_x = use_callback(move \|v\| …);` |
| `EventHandler::new(..)` / `Callback::new(..)` in a component body | **Yes**, and it allocates once per render | `use_callback` |
| `children: Element` | **Yes, always** (`VNode` eq is `Rc::ptr_eq`) | Keep children-taking components (`Modal`, `Menu`, `MenuItem`, `FormGroup`, `SelectField`) out of hot rows; put the costly part in a child with stable props |
| `x: ReadSignal<T>` passed as `sig.into()` **at top level** | No: memoize compares the peeked value, then `point_to` | Fine |
| `ReadSignal` **inside a struct prop**, or kept in a hook argument | **Yes**: a new boxed handle on each `into()` | Pass `Signal`/`Memo` |
| `Signal`, `Memo`, `Resource`, `Store`, `Callback` from `use_callback` | No: Copy handles compared by identity | Preferred |
| Owned data (`Vec<T>`, big structs) | Only if unequal, but it is cloned and deep-compared on every parent render | Pass an id plus a handle, and read inside the child |

## Subscriptions (`signals/src`, `hooks/src/use_resource.rs`)

- `x.read()`, `x()` or `x.cloned()` in a render body subscribes **that component**. A
  plain `fn render_*(…)` helper called from render subscribes its **caller**. To isolate
  reads, make the helper a `#[component]`.
- `peek()` reads without subscribing. Use it in handlers and for values that must not
  trigger a re-render.
- `Signal::set` always notifies, even with an equal value (`write.rs` `set` = `*write() =`).
  Guard hot writes with `if *s.peek() != v { s.set(v) }`. `Memo` notifies only when
  `new != old` (`memo.rs` `recompute`).
- `use_resource` tracks reads **anywhere in its future, even after an `.await`**,
  because every poll runs inside the reactive context. A dependency change cancels the
  old task and starts a new one. Use `peek()` for values that must not refetch.
- Plain props (`id: Uuid`) are **not** reactive inside `use_resource`, `use_memo` or
  `use_effect`. Use `use_reactive!(|id| …)` or remount the component with a key. Horae
  uses `for id in [id] { Page { key: "{id}", id } }`.

## Resources and loading (`hooks/src/use_resource.rs`, `fullstack-core/src/{loader,server_future}.rs`)

- `restart()` sets `state` to `Pending` and **keeps the old value**. The value is only
  `None` before the first load. Early-returning on `state() != Ready` therefore
  **unmounts the subtree and drops its local state** (filters, selections) on every
  reload. Gate only the first load (`read().is_none()`), and show stale data with
  `aria-busy` while a reload runs.
- Each resource starts its task on mount, so separate resources run in parallel. Awaiting
  A then B in one closure, or mounting a child only after A resolves, is a waterfall.
- Horae serves with streaming disabled, so SSR waits only for **suspended** futures
  (`fullstack-server/src/ssr.rs` → `wait_for_suspense`). A plain `use_resource` is not
  awaited: the SSR HTML holds the loading state, and the client fetches after hydration.
- `use_loader(..)?` suspends on the **first** load only, serializes the result into the
  HTML for hydration, keeps the value through `restart()`, and sends errors to the
  nearest `ErrorBoundary`. It needs `T: Serialize + DeserializeOwned + PartialEq`.
- `use_server_future(..)?` suspends **every time** its state is Pending, restarts
  included. The nearest `SuspenseBoundary` (the root one by default, from
  `core/src/root_wrapper.rs`) then swaps its content for the fallback. Use it only
  inside a tight local boundary.
- `use_action(f)` is for user-triggered mutations. It gives `.call(args)`, `.pending()`
  and `.value()`, and cancels the in-flight call when called again. A bare `spawn` in
  an onclick has none of that.

## Lists and nested state (`stores/src`, `examples/01-app-demos/todomvc_store.rs`)

- Give every list item a `key` with a stable id, never an index.
- For editable collections, use `#[derive(Store)]` + `use_store`. Keep items in a
  `HashMap`/`BTreeMap` by id, keep a `use_memo` of the ordered ids, and render each row
  as `Row { key: "{id}", store, id }`, which does `store.items().get(id)`. A row then
  re-renders only when its own entry changes. `Vec` stores are indexed: `remove`,
  `insert` and `retain` dirty every item after the first changed index.
- Build the ids memo from `store.items().iter().map(|(id, _)| id)` (shallow: it re-runs
  only on insert or remove). `store.items().read()` or `.cloned()` tracks **deep**, so
  that memo re-runs on every keystroke in any row.
- Without stores, a per-row `use_memo` over a `Signal<Vec<T>>` works, but it re-runs
  every memo on every write (O(n) per keystroke).

## Events, links, effects

- Call `prevent_default()` synchronously, before any `.await`. The web renderer checks
  it right after the handler returns (`web/src/dom.rs`), and forms are not prevented
  automatically.
- Use `Link { to: Route::… }` for internal navigation. A plain `a { href }` reloads the
  page and the WASM app, losing the shared session. `Link` sets `aria-current="page"`
  only when its href equals the current URL exactly (`router/src/components/link.rs`).
- `use_effect` takes no cleanup closure. Use `use_drop` for unmount cleanup. Tasks from
  `spawn` are cancelled on unmount.

## Common mistakes

| Mistake | Fix |
|---|---|
| "Callbacks are skipped in prop comparison" | False in 0.7.9: they are compared with `ptr_eq`. Use `use_callback` |
| Splitting out a row component that takes an inline closure or `children` | The split does nothing until its props are stable |
| `if r.state()() != Ready { return loading }` above stateful UI | Gate on `r.read().is_none()` |
| Polling loop `job.set(Some(j))` every tick | Write only on change |
| Treating SSR as "already has the data" | Only suspended futures (`use_loader`, `use_server_future`) are in the HTML |

## Before claiming a performance fix

Count renders in a VirtualDom test, **and** measure in a real browser at 4× CPU
throttle. A render-count test on the child does not show whether the parent's props
stayed equal.

## References

- `references/source-evidence.md`: the source excerpt behind each rule above.
- `references/horae-application.md`: where Horae breaks these rules today, with
  file/symbol and the fix, ranked.
