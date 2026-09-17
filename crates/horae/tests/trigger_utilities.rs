#![cfg(feature = "server")]

//! Shared controls keep their compact defaults when a page opts into utilities.

use dioxus::prelude::*;

#[path = "../src/components/combobox.rs"]
mod combobox;
#[path = "../src/components/menu.rs"]
mod menu;

use combobox::{ComboOption, Combobox};
use menu::Menu;

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
