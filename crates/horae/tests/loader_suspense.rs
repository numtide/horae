#![cfg(feature = "server")]

//! Dioxus 0.7.9, as the server render uses it: build, wait for suspense,
//! render. While it waits it polls only the tasks a component is suspended
//! on and drops the wake-up of any other task, and it takes a queued task
//! before re-running that task's component. A component with two
//! `use_loader`s suspends on the first, so the wait ends only if the second
//! read finishes after the first read's re-render has suspended on it. A
//! second read that finishes first, or together with the first, is never
//! polled again. One loader that joins the reads finishes in any order. A
//! server cannot choose when each read finishes, so `use_session_provider`
//! uses one loader, and a test that starts failing here means the rule can
//! be revisited.
//!
//! Each read waits on a channel the test completes, not on a timer: two
//! 10 ms timers usually fire together, but under load the second can fire
//! after the re-render, which is how CI once saw two loaders finish.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use dioxus::prelude::*;
use tokio::sync::oneshot;

/// The two reads' pending results, taken by the loaders on their first run.
#[derive(Clone)]
struct Reads([Rc<RefCell<Option<oneshot::Receiver<u32>>>>; 2]);

impl Reads {
    async fn read(&self, index: usize) -> Result<u32, ServerFnError> {
        let pending = self.0[index].borrow_mut().take();
        let value = pending.expect("each read runs once").await;
        Ok(value.expect("the test completes every read"))
    }
}

fn two_loaders(reads: Reads) -> Element {
    let first = use_loader({
        let reads = reads.clone();
        move || {
            let reads = reads.clone();
            async move { reads.read(0).await }
        }
    });
    let second = use_loader(move || {
        let reads = reads.clone();
        async move { reads.read(1).await }
    });
    let (first, second) = (first?, second?);
    rsx! { "{first} {second}" }
}

fn one_loader(reads: Reads) -> Element {
    let both = use_loader(move || {
        let reads = reads.clone();
        async move {
            let (first, second) = futures_util::join!(reads.read(0), reads.read(1));
            Ok::<_, ServerFnError>((first?, second?))
        }
    })?;
    let (first, second) = both();
    rsx! { "{first} {second}" }
}

/// The page as the server sends it, or `None` if the wait never ends. Each
/// step finishes its reads together; the wait runs every task it can and goes
/// idle before the first step and between steps.
async fn server_render(app: fn(Reads) -> Element, steps: &[&[usize]]) -> Option<String> {
    let ((first, first_read), (second, second_read)) = (oneshot::channel(), oneshot::channel());
    let mut senders = [Some(first), Some(second)];
    let reads = Reads([first_read, second_read].map(|r| Rc::new(RefCell::new(Some(r)))));
    let mut dom = VirtualDom::new_with_props(app, reads);
    dom.rebuild_in_place();
    // `futures_util::join!` polls the wait first and both futures on every
    // wake-up, so each yield lets the wait run until it is idle again.
    let finish_reads = async {
        for step in steps {
            tokio::task::yield_now().await;
            for &index in *step {
                let sender = senders[index].take().expect("each read finishes once");
                sender.send(index as u32 + 1).expect("the read is waiting");
            }
        }
    };
    let wait = tokio::time::timeout(Duration::from_secs(2), dom.wait_for_suspense());
    let (finished, ()) = futures_util::join!(wait, finish_reads);
    finished.ok()?;
    Some(dioxus::ssr::render(&dom))
}

#[tokio::test]
async fn one_loader_joining_the_reads_finishes_the_server_render_in_any_order() {
    for steps in [&[&[0][..], &[1]][..], &[&[1], &[0]], &[&[0, 1]]] {
        let page = server_render(one_loader, steps).await;
        assert_eq!(page.as_deref(), Some("1 2"), "{steps:?}");
    }
}

#[tokio::test]
async fn two_loaders_never_finish_the_server_render_when_the_second_read_finishes_first() {
    assert_eq!(server_render(two_loaders, &[&[1], &[0]]).await, None);
}

#[tokio::test]
async fn two_loaders_never_finish_the_server_render_when_the_reads_finish_together() {
    assert_eq!(server_render(two_loaders, &[&[0, 1]]).await, None);
}

#[tokio::test]
async fn two_loaders_finish_the_server_render_when_the_second_read_finishes_after_the_first() {
    let page = server_render(two_loaders, &[&[0], &[1]]).await;
    assert_eq!(page.as_deref(), Some("1 2"));
}
