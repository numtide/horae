//! The interface's text catalog, as the components reach it.

use dioxus::prelude::*;
// The UI tests include this module whole and use only part of it.
#[allow(unused_imports)]
pub(crate) use horae_i18n::{Lang, Tr, split_tag};

/// Call once in the app shell. The language is English until the language
/// setting exists; it will then come from the server before the first render,
/// so SSR and hydration pick the same catalog.
pub(crate) fn use_tr_provider() {
    use_context_provider(|| Tr::new(Lang::En));
}

/// The catalog of the language the page renders in. English when no provider
/// is above the caller, as in the component tests that mount one section.
#[expect(
    dead_code,
    reason = "no screen reads the catalog yet; the first migrated one removes this"
)]
pub(crate) fn use_tr() -> Tr {
    try_use_context::<Tr>().unwrap_or_default()
}
