use std::{cell::Cell, rc::Rc, time::Duration};

use dioxus::dioxus_core::Task;
use dioxus::prelude::*;

/// How long typing must pause before a search or filter runs.
pub const SEARCH_DEBOUNCE: Duration = Duration::from_millis(250);

/// A copy of `source` that catches up once `source` has been still for `delay`,
/// so a resource reading it runs once per pause instead of once per keystroke.
/// Each change cancels the previous wait; a return to the settled value is not
/// a change.
pub fn use_debounced<T: Clone + PartialEq + 'static>(
    source: Signal<T>,
    delay: Duration,
) -> ReadSignal<T> {
    let mut settled = use_signal(|| source.peek().clone());
    let waiting = use_hook(|| Rc::new(Cell::new(None::<Task>)));
    use_effect(move || {
        let value = source();
        if let Some(task) = waiting.take() {
            task.cancel();
        }
        if value != *settled.peek() {
            waiting.set(Some(spawn(async move {
                sleep(delay).await;
                settled.set(value);
            })));
        }
    });
    // Converting a signal boxes a new handle each time, and handles compare by
    // identity: convert once so props holding the result stay equal.
    use_hook(|| settled.into())
}

async fn sleep(delay: Duration) {
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::TimeoutFuture::new(delay.as_millis() as u32).await;
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(delay).await;
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::RefCell;

    use super::*;

    /// Runs effects and timers for longer than `SEARCH_DEBOUNCE`.
    pub(crate) async fn settle(dom: &mut VirtualDom) {
        let deadline = tokio::time::Instant::now() + SEARCH_DEBOUNCE * 2;
        while tokio::time::timeout_at(deadline, dom.wait_for_work())
            .await
            .is_ok()
        {
            dom.render_immediate_to_vec();
        }
    }

    /// The harness's source signal and every settled value it rendered.
    #[derive(Clone, Default)]
    struct Probe {
        source: Rc<Cell<Option<Signal<String>>>>,
        rendered: Rc<RefCell<Vec<String>>>,
    }

    impl PartialEq for Probe {
        fn eq(&self, other: &Self) -> bool {
            Rc::ptr_eq(&self.rendered, &other.rendered)
        }
    }

    fn harness(probe: Probe) -> Element {
        let source = use_signal(String::new);
        let settled = use_debounced(source, Duration::from_millis(40));
        probe.source.set(Some(source));
        probe.rendered.borrow_mut().push(settled());
        rsx! { "{settled}" }
    }

    fn mount() -> (VirtualDom, Probe, Signal<String>) {
        let probe = Probe::default();
        let mut dom = VirtualDom::new_with_props(harness, probe.clone());
        dom.rebuild_in_place();
        let source = probe.source.get().unwrap();
        (dom, probe, source)
    }

    fn type_text(dom: &mut VirtualDom, mut source: Signal<String>, text: &str) {
        dom.in_runtime(|| source.set(text.into()));
        dom.render_immediate_to_vec();
    }

    #[tokio::test]
    async fn a_burst_of_changes_settles_once_on_the_last_value() {
        let (mut dom, probe, source) = mount();
        for text in ["d", "de", "des"] {
            type_text(&mut dom, source, text);
        }
        settle(&mut dom).await;
        assert_eq!(*probe.rendered.borrow(), ["", "des"]);
    }

    #[test]
    fn a_re_render_returns_the_same_handle() {
        type Handles = Rc<RefCell<Vec<ReadSignal<String>>>>;
        let handles = Handles::default();
        let mut dom = VirtualDom::new_with_props(
            |handles: Handles| {
                let source = use_signal(String::new);
                handles
                    .borrow_mut()
                    .push(use_debounced(source, SEARCH_DEBOUNCE));
                rsx! {}
            },
            handles.clone(),
        );
        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate_to_vec();
        let handles = handles.borrow();
        assert_eq!(handles.len(), 2);
        assert!(handles[0] == handles[1]);
    }

    #[tokio::test]
    async fn returning_to_the_settled_value_cancels_the_pending_update() {
        let (mut dom, probe, source) = mount();
        type_text(&mut dom, source, "x");
        type_text(&mut dom, source, "");
        settle(&mut dom).await;
        assert_eq!(*probe.rendered.borrow(), [""]);
    }
}
