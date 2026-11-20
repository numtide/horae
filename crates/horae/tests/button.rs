#![cfg(feature = "server")]

//! `Button`, `IconButton` and `SplitButton`: native attributes pass through, the
//! type defaults to `button`, and a page re-render does not re-render a button
//! whose props are unchanged.

use dioxus::prelude::*;

#[path = "../src/components/button.rs"]
mod button;
#[allow(dead_code)]
#[path = "../src/i18n.rs"]
mod i18n;

use button::{Button, ButtonVariant, IconButton, SplitButton};

fn render(component: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(component);
    dom.rebuild_in_place();
    dioxus::ssr::render(&dom)
}

#[test]
fn button_defaults_to_a_primary_non_submitting_button() {
    let html = render(|| rsx! { Button { label: "Save" } });
    assert_eq!(
        html,
        r#"<button type="button" class="btn btn-primary">Save</button>"#
    );
}

#[test]
fn button_passes_native_attributes_through() {
    let html = render(|| {
        rsx! {
            Button {
                id: "save",
                r#type: "submit",
                disabled: true,
                aria_label: "Save the time zone",
                "data-testid": "save-zone",
                label: "Save",
            }
        }
    });
    for expected in [
        r#"id="save""#,
        r#"type="submit""#,
        r#"disabled=true"#,
        r#"aria-label="Save the time zone""#,
        r#"data-testid="save-zone""#,
    ] {
        assert!(html.contains(expected), "{expected} missing from {html}");
    }
    assert_eq!(html.matches("type=").count(), 1, "{html}");
}

#[test]
fn button_appends_caller_classes_and_sizes() {
    let html = render(|| {
        rsx! {
            Button { variant: ButtonVariant::DangerSolid, small: true, class: "w-full", label: "Delete" }
        }
    });
    assert!(
        html.contains(r#"class="btn btn-danger-solid btn-sm w-full""#),
        "{html}"
    );
    assert_eq!(html.matches("class=").count(), 1, "{html}");
}

#[test]
fn every_variant_has_its_class() {
    let html = render(|| {
        rsx! {
            Button { variant: ButtonVariant::Secondary, label: "Secondary" }
            Button { variant: ButtonVariant::Accent, label: "Accent" }
            Button { variant: ButtonVariant::Danger, label: "Danger" }
            Button { variant: ButtonVariant::Ghost, label: "Ghost" }
        }
    });
    for class in ["btn-secondary", "btn-accent", "btn-danger", "btn-ghost"] {
        assert!(html.contains(&format!("btn {class}\"")), "{class}: {html}");
    }
}

#[test]
fn button_draws_an_icon_before_its_label() {
    let html = render(|| rsx! { Button { label: "Start timer", span { "▶" } } });
    assert!(html.contains("<span>▶</span>Start timer"), "{html}");
}

#[test]
fn icon_button_is_named_by_its_label() {
    let html = render(|| rsx! { IconButton { label: "Start timer", id: "start", "▶" } });
    assert!(html.contains(r#"aria-label="Start timer""#), "{html}");
    assert!(html.contains(r#"class="btn btn-ghost btn-icon""#), "{html}");
    assert!(html.contains(r#"type="button""#), "{html}");
    assert!(html.contains(r#"id="start""#), "{html}");
}

#[test]
fn icon_button_label_replaces_a_passed_aria_label() {
    let html = render(|| rsx! { IconButton { label: "Start timer", aria_label: "Other", "▶" } });
    assert_eq!(html.matches("aria-label=").count(), 1, "{html}");
    assert!(html.contains(r#"aria-label="Start timer""#), "{html}");
}

#[test]
fn disabled_split_button_disables_its_caret() {
    let html = render(|| rsx! { SplitButton { label: "Submit week", disabled: true } });
    let caret = html.split("<button").nth(2).unwrap();
    assert!(caret.contains("disabled=true"), "{html}");

    let html = render(|| rsx! { SplitButton { label: "Submit week" } });
    let caret = html.split("<button").nth(2).unwrap();
    assert!(!caret.contains("disabled"), "{html}");
}

#[test]
fn split_button_gives_attributes_to_the_main_action() {
    let html = render(|| rsx! { SplitButton { label: "Submit week", id: "submit-week" } });
    let main = html.split("<button").nth(1).unwrap();
    assert!(main.contains(r#"id="submit-week""#), "{html}");
    assert!(main.contains(r#"type="button""#), "{html}");
    let caret = html.split("<button").nth(2).unwrap();
    assert!(caret.contains(r#"aria-label="More actions""#), "{html}");
    assert!(!caret.contains("submit-week"), "{html}");
}

/// Re-renders the page (`ScopeId::APP`) with no change and reports whether the
/// button below it rendered again. A render produces a new root node, and nodes
/// compare by pointer, so an unchanged root node means the button was skipped.
fn button_rerenders_with_page(page: fn() -> Element) -> bool {
    let mut dom = VirtualDom::new(page);
    dom.rebuild_in_place();
    // The page is the only component above the button, so the button is the next scope.
    let button = ScopeId(ScopeId::APP.0 + 1);
    let before = dom.get_scope(button).unwrap().root_node().clone();
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate_to_vec();
    let after = dom.get_scope(button).unwrap().root_node().clone();
    before != after
}

#[test]
fn label_and_use_callback_button_skips_page_rerenders() {
    fn page() -> Element {
        let onclick = use_callback(|_| {});
        rsx! { Button { id: "save", disabled: false, label: "Save", onclick } }
    }
    assert!(!button_rerenders_with_page(page));
}

#[test]
fn button_without_a_handler_skips_page_rerenders() {
    fn page() -> Element {
        rsx! { Button { label: "Save" } }
    }
    assert!(!button_rerenders_with_page(page));
}

#[test]
fn split_button_with_use_callback_skips_page_rerenders() {
    fn page() -> Element {
        let onclick = use_callback(|_| {});
        let ontrigger = use_callback(|_| {});
        rsx! { SplitButton { label: "Submit week", onclick, ontrigger } }
    }
    assert!(!button_rerenders_with_page(page));
}

// The two cases below are what the rule is about: the check above would pass
// for any button if it could not see a re-render.

#[test]
fn inline_closure_rerenders_the_button() {
    fn page() -> Element {
        rsx! { Button { label: "Save", onclick: move |_| {} } }
    }
    assert!(button_rerenders_with_page(page));
}

#[test]
fn children_rerender_the_button() {
    fn page() -> Element {
        rsx! { Button { label: "Start timer", span { "▶" } } }
    }
    assert!(button_rerenders_with_page(page));
}
