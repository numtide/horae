#![cfg(feature = "server")]

//! `ConfirmDialog`: the tone picks the tile and the confirm button, the footer
//! keeps a caller's order, and a confirmation word holds the button disabled.

use dioxus::prelude::*;

#[allow(dead_code)]
#[path = "../src/components/button.rs"]
mod button;
#[path = "../src/components/confirm_dialog.rs"]
mod confirm_dialog;
#[path = "../src/components/icons.rs"]
mod icons;
#[allow(dead_code)]
#[path = "../src/components/modal.rs"]
mod modal;

use confirm_dialog::{ConfirmDialog, ConfirmTone};

fn render(component: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(component);
    dom.rebuild_in_place();
    dioxus::ssr::render(&dom)
}

/// The opening tag of the button with this id, and where it starts.
fn button<'a>(html: &'a str, id: &str) -> (usize, &'a str) {
    let start = html
        .find(&format!(r#"<button id="{id}""#))
        .unwrap_or_else(|| panic!("no {id}: {html}"));
    let end = start + html[start..].find('>').unwrap();
    (start, &html[start..end])
}

#[test]
fn danger_confirms_with_the_solid_red_button_first() {
    let html = render(|| {
        rsx! {
            ConfirmDialog { id: "delete", labelledby: "delete-title", open: true,
                title: "Delete it?", confirm_label: "Delete",
                on_confirm: |_| {}, on_dismiss: |_| {},
                p { "It cannot be undone." }
            }
        }
    });
    assert!(html.contains("confirm-icon confirm-icon-danger"), "{html}");
    assert!(
        html.contains(r#"<h2 id="delete-title""#),
        "the title names the dialog: {html}"
    );
    let (confirm_at, confirm) = button(&html, "delete-confirm");
    let (cancel_at, cancel) = button(&html, "delete-cancel");
    assert!(confirm.contains("btn btn-danger-solid"), "{confirm}");
    assert!(cancel.contains("btn btn-secondary"), "{cancel}");
    assert!(
        confirm_at < cancel_at,
        "confirm comes first by default: {html}"
    );
    assert!(!html.contains("disabled=true"), "{html}");
}

#[test]
fn neutral_uses_primary_and_keeps_a_cancel_first_footer() {
    let html = render(|| {
        rsx! {
            ConfirmDialog { id: "archive", labelledby: "archive-title", open: true,
                title: "Archive 2 projects?", confirm_label: "Archive projects",
                tone: ConfirmTone::Neutral, cancel_first: true, cancel_label: "Keep them",
                on_confirm: |_| {}, on_dismiss: |_| {},
            }
        }
    });
    assert!(html.contains("confirm-icon confirm-icon-neutral"), "{html}");
    let (confirm_at, confirm) = button(&html, "archive-confirm");
    let (cancel_at, _) = button(&html, "archive-cancel");
    assert!(confirm.contains("btn btn-primary"), "{confirm}");
    assert!(html.contains(">Keep them<"), "{html}");
    assert!(cancel_at < confirm_at, "{html}");
}

#[test]
fn a_confirmation_word_disables_confirm_until_typed() {
    let html = render(|| {
        rsx! {
            ConfirmDialog { id: "wipe", labelledby: "wipe-title", open: true,
                title: "Delete the workspace?", confirm_label: "Delete workspace",
                confirm_word: "fieldstone".to_string(),
                on_confirm: |_| {}, on_dismiss: |_| {},
            }
        }
    });
    assert!(html.contains(r#"for="wipe-word""#), "{html}");
    assert!(
        button(&html, "wipe-confirm").1.contains("disabled=true"),
        "{html}"
    );
    assert!(
        !button(&html, "wipe-cancel").1.contains("disabled=true"),
        "{html}"
    );
}

#[test]
fn busy_disables_both_buttons() {
    let html = render(|| {
        rsx! {
            ConfirmDialog { id: "busy", labelledby: "busy-title", open: true, busy: true,
                title: "Disconnect?", confirm_label: "Disconnecting…",
                on_confirm: |_| {}, on_dismiss: |_| {},
            }
        }
    });
    assert!(
        button(&html, "busy-confirm").1.contains("disabled=true"),
        "{html}"
    );
    assert!(
        button(&html, "busy-cancel").1.contains("disabled=true"),
        "{html}"
    );
}
