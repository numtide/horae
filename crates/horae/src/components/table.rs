use dioxus::prelude::*;

/// The frame of every table (`DESIGN.md`, *Data Tables*): a bordered scroller
/// around the `table` passed as children. The options are the pattern's only
/// variants, so screens never style a table frame of their own.
#[component]
pub fn DataTable(
    /// Names the scroller and makes it a focusable region, so a keyboard can
    /// scroll a table that is wider or taller than its frame.
    #[props(into)]
    label: Option<String>,
    /// Inside a card that is already the frame: only a rule under its title.
    #[props(default)]
    flush: bool,
    /// Dense, scanned data (detailed reports, importer errors, invoice lines).
    #[props(default)]
    compact: bool,
    /// Bands every other row, for row-by-row lists with few columns.
    #[props(default)]
    striped: bool,
    /// Long lists: the scroller takes a maximum height and the header stays.
    #[props(default)]
    sticky_header: bool,
    /// A reload is under way: the rows stay, dimmed and `aria-busy`.
    #[props(default)]
    busy: bool,
    children: Element,
) -> Element {
    let mut class = String::from("table-container");
    for (on, name) in [
        (flush, " table-flush"),
        (compact, " table-compact"),
        (striped, " table-striped"),
        (sticky_header, " table-sticky"),
    ] {
        if on {
            class.push_str(name);
        }
    }
    let region = label.is_some();
    rsx! {
        div {
            class,
            role: region.then_some("region"),
            aria_label: label,
            tabindex: region.then_some("0"),
            aria_busy: busy.then_some("true"),
            {children}
        }
    }
}

/// Skeleton rows under the table's real header while its rows load for the
/// first time. `label` is the status a screen reader hears instead.
#[component]
pub fn TableSkeleton(
    columns: usize,
    #[props(into)] label: String,
    #[props(default = 3)] rows: usize,
) -> Element {
    rsx! {
        for row in 0..rows {
            tr { key: "{row}", class: "table-skeleton",
                for cell in 0..columns {
                    td { key: "{cell}",
                        if row == 0 && cell == 0 {
                            span { class: "sr-only", role: "status", "{label}" }
                        }
                        span { class: "skeleton-bar", aria_hidden: "true" }
                    }
                }
            }
        }
    }
}

/// The one row a table body shows instead of its rows when it is empty,
/// matches nothing or failed to load: a centred cell spanning every column,
/// holding an inset `EmptyState`, `ErrorState` or `LoadError`.
#[component]
pub fn TableMessage(columns: usize, children: Element) -> Element {
    rsx! {
        tr { class: "table-state",
            td { colspan: "{columns}", {children} }
        }
    }
}

/// A first column cell: the record's name (`children`, plain text or a link)
/// in bold, and an optional secondary line under it.
#[component]
pub fn NameCell(secondary: Option<Element>, children: Element) -> Element {
    rsx! {
        td { class: "name-cell",
            span { class: "name-cell-name", {children} }
            if let Some(secondary) = secondary {
                span { class: "name-cell-secondary", {secondary} }
            }
        }
    }
}

/// The bar of actions for the selected rows of a list, floating at the bottom
/// of the screen on the menu surface. Render it only while rows are selected.
#[component]
pub fn BulkBar(
    /// How many rows are selected.
    count: usize,
    /// Empties the selection.
    on_clear: EventHandler<()>,
    /// The actions, as small secondary buttons.
    children: Element,
) -> Element {
    let tr = crate::i18n::use_tr();
    rsx! {
        div { class: "bulk-bar", role: "group", aria_label: tr.bulk_bar_label(),
            span { class: "bulk-bar-count", role: "status", {tr.bulk_bar_selected(count)} }
            span { class: "bulk-bar-rule", aria_hidden: "true" }
            {children}
            button { r#type: "button", class: "btn btn-ghost btn-sm", onclick: move |_| on_clear.call(()),
                {tr.bulk_bar_clear()}
            }
        }
    }
}
