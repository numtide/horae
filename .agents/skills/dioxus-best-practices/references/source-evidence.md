# Source evidence (Dioxus v0.7.9)

Every rule in `SKILL.md` is backed by one of the excerpts below. Paths are relative to
`packages/` in `github.com/DioxusLabs/dioxus` at tag `v0.7.9` (commit `3e43ffa`). The
same files sit in `~/.cargo/registry/src/*/dioxus-*-0.7.9/`. Horae's
`vendor/dioxus-fullstack-0.7.9` only backports a response-body error fix (see
`vendor/PATCHES.md`), so none of the behaviour below depends on it.

To re-check a claim after a Dioxus upgrade, clone the new tag and read the same symbol.

## Props memoization

**`core-macro/src/component.rs`**: the generated `PartialEq` for a `#[component]`
props struct compares every field:

```rust
fn eq(&self, other: &Self) -> bool {
    #( self.#struct_field_idents == other.#struct_field_idents && )*
    true
}
```

**`core-macro/src/props/mod.rs` `memoize_impl`**, for props with no signal fields:

```rust
let equal = self == new;
#move_event_handlers            // self.#name.__point_to(&new.#name);
if !equal { /* copy regular fields */ }
equal
```

Handlers are re-pointed in place after the comparison, but the function still returns
`self == new`, and that comparison includes the handler fields.

When there are signal fields (`looks_like_signal_type`: `ReadSignal` at top level), the
comparison is `(&old_value).compare(&&new_value)` on the **peeked values**, followed by
`point_to` plus `mark_dirty` when the values differ. That is why a top-level
`x: ReadSignal<T>` prop built with `.into()` stays stable.

**`core/src/events.rs`**:

```rust
impl<Args, Ret> PartialEq for Callback<Args, Ret> {
    fn eq(&self, other: &Self) -> bool {
        self.callback.ptr_eq(&other.callback) && self.origin == other.origin
    }
}
/// Create a new [`Callback`] from an [`FnMut`]. The callback is owned by the current scope and will be dropped when the scope is dropped.
/// This should not be called directly in the body of a component because it will not be dropped until the component is dropped.
pub fn new(...) -> Self { let owner = current_owner(); let callback = owner.insert_rc(...); ... }
```

An inline closure prop reaches `Callback::new` through `SuperFrom` on every render, so
each render produces a new pointer (unequal) and a new owned box.

**`hooks/src/use_callback.rs`**: one handle per hook slot. The closure is swapped with
`inner.replace(Box::new(callback))` on later renders, so the handle stays equal.

**`core/src/nodes.rs`**:

```rust
impl PartialEq for VNode { fn eq(&self, other: &Self) -> bool { Rc::ptr_eq(&self.vnode, &other.vnode) } }
```

`children: Element` is always a new `Rc` after the parent re-renders.

**`signals/src/boxed.rs`**: `ReadSignal { value: CopyValue<Box<..>> }`, and
`impl PartialEq for ReadSignal { fn eq(..) { self.value == other.value } }`, a
comparison of the box handle. `From<Signal<T>> for ReadSignal<T>` calls
`Self::new_maybe_sync(value)`, which creates a new `CopyValue` (in the current owner)
on each call. Two `.into()` conversions of the same signal therefore compare unequal.
That only harms a prop nested inside a struct or held in a hook argument, because the
macro's top-level signal handling above compares values instead. `Resource::state()` and
`Resource::value()` return `self.state.into()`, so each call in a render body allocates
a small handle that lives until the component unmounts. Prefer reading the resource
itself (`r.read()`) where you can.

## Signals and memos

**`signals/src/write.rs`**: `fn set(&mut self, value) { *self.write() = value; }`.
Every write marks the signal's subscribers dirty, whatever the value.

**`signals/src/memo.rs` `recompute`**: `if new_value != *peak { … notify … }`. A memo
notifies only on change, which is why `use_memo` requires `R: PartialEq`
(`hooks/src/use_memo.rs`).

**`hooks/src/use_reactive.rs`**: `use_reactive(non_reactive_data, closure)` keeps the
last value in a signal and writes it when the value changes. The macro form is
`use_reactive!(|a, b| …)`. Use it for plain props inside memos, effects and resources.

## use_resource

**`hooks/src/use_resource.rs`**:

```rust
let cb = use_callback(move |_| {
    state.set(UseResourceState::Pending);
    let fut = rc.reset_and_run_in(&mut future);
    spawn(async move {
        // Run each poll in the context of the reactive scope
        let res = future::poll_fn(|cx| rc.run_in(|| fut.poll_unpin(cx))).await;
        state.set(UseResourceState::Ready);
        value.set(Some(res));
    })
});
// dependency changes: task.write().cancel(); task.set(cb(()));
pub fn restart(&mut self) { self.task.write().cancel(); let new_task = self.callback.call(()); self.task.set(new_task); }
pub fn pending(&self) -> bool { matches!(*self.state.peek(), UseResourceState::Pending) }  // peek: does not subscribe
```

This gives three properties:
- reads anywhere in the future, including after `.await`, are tracked;
- a dependency change cancels the old task, so a superseded response cannot land;
- `restart()` sets Pending and leaves `value` untouched until the new result arrives.

## SSR, use_server_future, use_loader, boundaries

**`fullstack-server/src/ssr.rs`**:

```rust
// If streaming is disabled, wait for the virtual dom to finish all suspense work
if streaming_mode == StreamingMode::Disabled { virtual_dom.wait_for_suspense().await; }
```

Horae calls `.serve_dioxus_application(ServeConfig::new(), app::App)` in
`crates/horae/src/main.rs`, with streaming at its default (disabled). Only futures that
**suspended** are awaited before the HTML is sent.

**`fullstack-core/src/server_future.rs` `use_server_future`**: it wraps `use_resource`,
inserts the result into the serialize context on the server (`storage_entry.insert`),
and on the web client takes `initial_web_result` from the HTML on the first run. Then:

```rust
if resource.state().cloned() == UseResourceState::Pending { … return Err(suspend(task).unwrap_err()); }
```

It suspends on every Pending state, restarts included.

**`core/src/suspense/component.rs`** (SuspenseBoundary diff): "We have no suspended
nodes, but we just became suspended. Move the children to the background", followed by
"Set the last rendered node to the new suspense placeholder". In other words, a
re-suspend shows the fallback.

**`core/src/root_wrapper.rs`**: "We wrap the root scope in a component that renders it
inside a default ErrorBoundary and SuspenseBoundary". An unbounded suspend or error
therefore falls back at the root.

**`fullstack-core/src/loader.rs` `use_loader`**:
- it requires `T: 'static + PartialEq + Serialize + DeserializeOwned` and
  `E: Into<CapturedError>`;
- `loader_state` starts as `Pending` and is set to `Ready` or `Failed` only by the
  future, so `restart()` never puts it back to Pending, and the component keeps
  rendering the old value;
- `impl From<Loading> for RenderError` turns Pending into `Suspended` and Failed into
  `RenderError::Error`, so `use_loader(..)?` routes errors to the nearest
  `ErrorBoundary`;
- `CapturedError` covers any `E: Into<anyhow::Error>` (`core/src/render_error.rs`).
- `Loader::restart(&mut self)` reruns the future; `Loader` is `Copy` and readable like a
  signal (`loader.read()`).

**`core/src/error_boundary.rs`**:
- `ErrorBoundary { handle_error: |errors: ErrorContext| rsx! { … }, children }`;
- `ErrorContext::error() -> Option<CapturedError>`;
- `ErrorContext::clear_errors()` resets the boundary so its children render again. For
  a retry, call it in the handler after fixing the cause, or remount the section with a
  new `key`.

## use_action

**`hooks/src/use_action.rs`**:

```rust
let callback = use_callback(move |input| {
    if let Some(task) = task.take() { task.cancel(); }   // re-call cancels in-flight work
    let result = user_fn.call(input);
    let new_task = spawn(async move { state.set(ActionState::Pending); match result.await { Ok(res) => { value.set(Some(res)); state.set(Ready) } Err(err) => { error.set(Some(err.into())); state.set(Errored) } } });
    …
});
pub fn value(&self) -> Option<Result<ReadSignal<O>, CapturedError>>
pub fn pending(&self) -> bool
```

You call it with `action.call(args…)`, which returns an awaitable `Dispatching<()>`. It
resolves only after the result is stored, because `tx.send(())` runs after the
`value`/`state` writes. `action.call(x).await; if let Some(Ok(_)) = action.value() { … }`
therefore sees the outcome of this call.
`Action` is `Copy`, and its `PartialEq` compares the callback handle, so it is a
stable prop.

## Stores

**`stores/src/impls/vec.rs`**:
- `push` only marks the length dirty (`mark_dirty_shallow`);
- `remove` and `insert` call `mark_dirty_at_and_after_index(index)`;
- `retain` dirties everything after the first removed index.

**`stores/src/impls/slice.rs`**: `iter`, `len` and `get` "only track the shallow state"
of the slice. `get(i)` returns a store that "will only update when the item at the
index changes".

**`stores/src/impls/{hashmap,btreemap}.rs`**: `get<Q>(self, key: Q)` takes the key
by value, `insert(key, value)` and `remove(&key)` work per key, and `len`, `is_empty`
and `iter` "track the store shallowly and only cause re-runs when items are added or
removed from the map, not when existing values are modified" (`selector().track_shallow()`).

**`stores/src/scope.rs`**: `Readable::try_read_unchecked` calls `self.track()`, which
is `self.store.track_deep(&self.path)`. A plain `.read()`, `.cloned()` or call on a
store subscribes to every nested field. Use `iter()`/`len()` when you only need the
keys.

**`examples/01-app-demos/todomvc_store.rs`** is the canonical pattern:

```rust
#[derive(Store, PartialEq, Clone, Debug)]
struct TodoState { todos: HashMap<u32, TodoItem>, filter: FilterState }
#[derive(Store, PartialEq, Clone, Debug)]
struct TodoItem { checked: bool, contents: String }

let todos = use_store(|| TodoState { … });
let filtered_todos = use_memo(move || todos.active_items());   // ids only
for id in filtered_todos() { TodoEntry { key: "{id}", id, todos } }

#[component]
fn TodoEntry(mut todos: Store<TodoState>, id: u32) -> Element {
    // "When we get an item out of the store, it will only subscribe to that specific item."
    let entry = todos.todos().get(id).unwrap();
    … oninput: move |evt| entry.contents().set(evt.value()) …
}
```

A `#[store] impl<Lens> Store<T, Lens> { fn …(&self) / (&mut self) }` block adds methods
to the store. Stores are part of the `signals` feature (`dioxus/Cargo.toml`:
`signals = ["dep:dioxus-signals", "dep:dioxus-stores"]`), which Horae already enables.

## Events and links

**`web/src/dom.rs`**: after `runtime.handle_event(..)` comes
`let prevent_default = !event.default_action_enabled(); if prevent_default { web_sys_event.prevent_default(); }`.
The check runs once, synchronously, as soon as the handler returns. The web renderer
never prevents form submission by default.

**`router/src/components/link.rs`**:
- the click `action` returns early for modifier keys, non-primary buttons, `new_tab`
  and external targets;
- otherwise it calls `event.prevent_default(); router.push_any(to)`;
- `aria_current = (href == current_url).then_some("page")`, which is an exact match.

A raw `a { href }` takes none of these paths, so the browser performs a full
navigation.

## Effects and cleanup

**`hooks/src/use_effect.rs`**: `use_effect(callback: impl FnMut() + 'static) -> Effect`,
with no return value used. Reads are tracked by its `ReactiveContext`. For cleanup use
`use_drop` (`core/src/global_context.rs`) or `use_on_unmount`
(`hooks/src/use_on_destroy.rs`).
