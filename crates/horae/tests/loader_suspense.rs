#![cfg(feature = "server")]

//! Dioxus 0.7.9, as the server render uses it: build, wait for suspense,
//! render. Two `use_loader`s in one component never let that wait finish;
//! one loader that joins the reads does. `use_session_provider` relies on
//! this, and a test that starts failing here means the rule can be revisited.

use std::time::Duration;

use dioxus::prelude::*;

async fn read(value: u32) -> Result<u32, ServerFnError> {
    tokio::time::sleep(Duration::from_millis(10)).await;
    Ok(value)
}

fn two_loaders() -> Element {
    let first = use_loader(|| read(1));
    let second = use_loader(|| read(2));
    let (first, second) = (first?, second?);
    rsx! { "{first} {second}" }
}

fn one_loader() -> Element {
    let both = use_loader(|| async {
        let (first, second) = futures_util::join!(read(1), read(2));
        Ok::<_, ServerFnError>((first?, second?))
    })?;
    let (first, second) = both();
    rsx! { "{first} {second}" }
}

/// The page as the server sends it, or `None` if the wait never ends.
async fn server_render(app: fn() -> Element) -> Option<String> {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    tokio::time::timeout(Duration::from_secs(2), dom.wait_for_suspense())
        .await
        .ok()?;
    Some(dioxus::ssr::render(&dom))
}

#[tokio::test]
async fn one_loader_joining_the_reads_finishes_the_server_render() {
    assert_eq!(server_render(one_loader).await.as_deref(), Some("1 2"));
}

#[tokio::test]
async fn two_loaders_in_one_component_never_finish_the_server_render() {
    assert_eq!(server_render(two_loaders).await, None);
}
