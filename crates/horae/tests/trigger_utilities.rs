#![cfg(feature = "server")]

//! Shared controls keep their compact defaults when a page opts into utilities.

use dioxus::prelude::*;

#[path = "../src/components/combobox.rs"]
mod combobox;
#[path = "../src/components/controls.rs"]
mod controls;
#[path = "../src/components/menu.rs"]
mod menu;

use combobox::{ComboOption, Combobox};
use controls::Checkbox;
use menu::Menu;

#[test]
fn checkbox_keeps_labelled_unchecked_defaults() {
    let html = render(|| rsx! { Checkbox { checked: false, label: "Include archived" } });
    assert!(html.contains("class=\"choice\""), "{html}");
    assert!(html.contains("aria-checked=\"false\""), "{html}");
    assert!(html.contains("<span>Include archived</span>"), "{html}");
    assert!(!html.contains("disabled=\"true\""), "{html}");
}

#[test]
fn compact_mixed_checkbox_keeps_an_accessible_name() {
    let html = render(|| {
        rsx! {
            Checkbox { checked: false, mixed: true, compact: true, label: "Select all visible projects" }
        }
    });
    assert!(html.contains("aria-checked=\"mixed\""), "{html}");
    assert!(
        html.contains("aria-label=\"Select all visible projects\""),
        "{html}"
    );
    assert!(html.contains("choice checked"), "{html}");
    assert!(
        !html.contains("<span>Select all visible projects</span>"),
        "{html}"
    );
}

#[test]
fn disabled_checked_checkbox_is_natively_disabled() {
    let html = render(|| rsx! { Checkbox { checked: true, disabled: true, label: "Busy" } });
    assert!(html.contains("disabled"), "{html}");
    assert!(html.contains("aria-checked=\"true\""), "{html}");
    assert!(html.contains("✓"), "{html}");
}

fn render(component: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(component);
    dom.rebuild_in_place();
    dioxus::ssr::render(&dom)
}

#[test]
fn menu_keeps_compact_defaults() {
    let html = render(|| {
        rsx! {
            Menu { id: "default-menu", label: "Actions" }
        }
    });
    assert!(html.contains("btn btn-secondary btn-sm"), "{html}");
    assert!(!html.contains("text-sm"), "{html}");
    assert!(!html.contains("disabled"), "{html}");
}

#[test]
fn menu_can_disable_its_native_trigger() {
    let html = render(|| rsx! { Menu { id: "disabled-menu", label: "Actions", disabled: true } });
    let trigger = html
        .split("<button")
        .nth(1)
        .unwrap()
        .split('>')
        .next()
        .unwrap();
    assert!(trigger.contains("disabled"), "{html}");
}

#[test]
fn combobox_keeps_compact_defaults() {
    let html = render(|| {
        rsx! {
            Combobox {
                options: vec![ComboOption::grouped("client", "Client", "Clients")],
                value: "", placeholder: "Client",
            }
        }
    });
    assert!(html.contains("btn btn-secondary btn-sm"), "{html}");
    assert!(!html.contains("text-sm"), "{html}");
}

#[test]
fn menu_composes_trigger_utilities() {
    let html = render(|| {
        rsx! {
            Menu {
                id: "custom-menu", label: "Actions",
                trigger_class: "text-sm px-4",
            }
        }
    });
    assert!(
        html.contains("btn btn-secondary btn-sm text-sm px-4"),
        "{html}"
    );
}

#[test]
fn combobox_composes_trigger_utilities() {
    let html = render(|| {
        rsx! {
            Combobox {
                options: vec![], value: "", placeholder: "Client",
                trigger_class: "text-sm px-4",
            }
        }
    });
    assert!(
        html.contains("btn btn-secondary btn-sm text-sm px-4"),
        "{html}"
    );
}
