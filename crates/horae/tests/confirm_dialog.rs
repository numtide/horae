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
fn neutral_uses_primary_with_its_own_cancel_label() {
    let html = render(|| {
        rsx! {
            ConfirmDialog { id: "archive", labelledby: "archive-title", open: true,
                title: "Archive 2 projects?", confirm_label: "Archive projects",
                tone: ConfirmTone::Neutral, cancel_label: "Keep them",
                on_confirm: |_| {}, on_dismiss: |_| {},
            }
        }
    });
    assert!(html.contains("confirm-icon confirm-icon-neutral"), "{html}");
    let (confirm_at, confirm) = button(&html, "archive-confirm");
    let (cancel_at, _) = button(&html, "archive-cancel");
    assert!(confirm.contains("btn btn-primary"), "{confirm}");
    assert!(html.contains(">Keep them<"), "{html}");
    assert!(
        confirm_at < cancel_at,
        "the primary action always comes first: {html}"
    );
}

#[test]
fn modal_draws_primary_then_secondary_then_cancel() {
    let html = render(|| {
        let save = use_callback(|()| {});
        let remove = use_callback(|()| {});
        rsx! {
            // Declared in any order, drawn in the fixed one.
            modal::Modal { id: "entry", labelledby: "entry-title", open: true,
                cancel: modal::ModalCancel::new("Close"),
                secondary: modal::ModalAction::click("Delete", remove).variant(button::ButtonVariant::Danger),
                primary: modal::ModalAction::click("Save", save).id("entry-save"),
                on_dismiss: |_| {},
            }
        }
    });
    let save = html.find(">Save<").expect(&html);
    let delete = html.find(">Delete<").expect(&html);
    let close = button(&html, "entry-cancel").0;
    assert!(save < delete && delete < close, "{html}");
    assert!(
        button(&html, "entry-save").1.contains("btn btn-primary"),
        "{html}"
    );
    assert_eq!(html.matches("class=\"modal-actions\"").count(), 1, "{html}");
}

#[test]
fn danger_opens_on_cancel_and_the_body_describes_the_dialog() {
    let danger = render(|| {
        rsx! {
            ConfirmDialog { id: "drop", labelledby: "drop-title", open: true,
                title: "Delete it?", confirm_label: "Delete",
                on_confirm: |_| {}, on_dismiss: |_| {},
                p { "It cannot be undone." }
            }
        }
    });
    assert!(
        button(&danger, "drop-cancel").1.contains("autofocus"),
        "{danger}"
    );
    assert!(
        !button(&danger, "drop-confirm").1.contains("autofocus"),
        "{danger}"
    );
    assert!(
        danger.contains(r#"aria-describedby="drop-body""#),
        "{danger}"
    );
    assert!(
        danger.contains(r#"<div id="drop-body" class="confirm-body">"#),
        "{danger}"
    );
    let neutral = render(|| {
        rsx! {
            ConfirmDialog { id: "keep", labelledby: "keep-title", open: true,
                tone: ConfirmTone::Neutral, title: "Archive it?", confirm_label: "Archive",
                on_confirm: |_| {}, on_dismiss: |_| {},
            }
        }
    });
    assert!(!neutral.contains("autofocus"), "{neutral}");
}

#[test]
fn confirm_disabled_holds_the_button() {
    let html = render(|| {
        rsx! {
            ConfirmDialog { id: "wait", labelledby: "wait-title", open: true, confirm_disabled: true,
                title: "Delete it?", confirm_label: "Delete",
                on_confirm: |_| {}, on_dismiss: |_| {},
            }
        }
    });
    assert!(
        button(&html, "wait-confirm").1.contains("disabled=true"),
        "{html}"
    );
    assert!(
        !button(&html, "wait-cancel").1.contains("disabled=true"),
        "{html}"
    );
}

#[test]
fn closed_it_renders_no_buttons() {
    let html = render(|| {
        rsx! {
            ConfirmDialog { id: "shut", labelledby: "shut-title", open: false,
                title: "Delete it?", confirm_label: "Delete",
                on_confirm: |_| {}, on_dismiss: |_| {},
            }
        }
    });
    assert!(!html.contains("<button"), "{html}");
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
