//! Horae's interface text, in Fluent catalogs under `locales/<lang>/horae.ftl`.
//!
//! Code never names a message by a string: `build.rs` turns every English message
//! into a method on [`Tr`], so `tr.api_token_created(name)` fails to compile when
//! the message or its `$name` variable is gone. English is the source and the
//! fallback; a translation that lacks a message shows the English one.
//!
//! Both the server (SSR, mails, PDFs) and the WASM client build this crate, and
//! the catalogs are compiled in, so the two sides render the same text for the
//! same [`Lang`] with no fetch before the first paint.

use std::sync::LazyLock;

use fluent_bundle::FluentArgs;
use fluent_bundle::FluentResource;
pub use fluent_bundle::FluentValue;
use fluent_bundle::concurrent::FluentBundle;

include!(concat!(env!("OUT_DIR"), "/messages.rs"));

/// One parsed bundle per language, in the order of [`Lang::ALL`].
static BUNDLES: LazyLock<Vec<FluentBundle<FluentResource>>> = LazyLock::new(|| {
    Lang::ALL
        .iter()
        .zip(SOURCES)
        .map(|(lang, source)| {
            let resource = FluentResource::try_new((*source).to_string()).unwrap_or_else(|_| {
                panic!("the {} catalog parses: build.rs checked it", lang.tag())
            });
            let id = lang.tag().parse().expect("a valid language tag");
            let mut bundle = FluentBundle::new_concurrent(vec![id]);
            // Bidi isolation marks around placeables matter for right-to-left
            // languages only, and would change today's English output.
            bundle.set_use_isolating(false);
            bundle
                .add_resource(resource)
                .expect("message ids are unique: build.rs checked them");
            bundle
        })
        .collect()
});

/// The catalog of one language. Copy it into components freely.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tr {
    lang: Lang,
}

impl Tr {
    pub const fn new(lang: Lang) -> Self {
        Self { lang }
    }

    pub fn lang(self) -> Lang {
        self.lang
    }

    // The generated methods are its only callers, and a catalog may have none.
    #[allow(dead_code)]
    fn format(&self, id: &str, args: Option<&FluentArgs>) -> String {
        let index = Lang::ALL.iter().position(|l| *l == self.lang).unwrap_or(0);
        let (bundle, message) = match BUNDLES[index].get_message(id) {
            Some(message) => (&BUNDLES[index], message),
            None => (
                &BUNDLES[0],
                BUNDLES[0]
                    .get_message(id)
                    .expect("every method has an English message"),
            ),
        };
        let pattern = message.value().expect("build.rs requires a value");
        let mut errors = Vec::new();
        let text = bundle.format_pattern(pattern, args, &mut errors);
        debug_assert!(errors.is_empty(), "`{id}`: {errors:?}");
        text.into_owned()
    }
}

/// Splits a message around its one `<tag>…</tag>`, for the screen to wrap the
/// middle in an element: `("… described in ", "its OpenAPI document", ".")`.
/// A message without the tag comes back whole, as the first part.
pub fn split_tag<'a>(text: &'a str, tag: &str) -> (&'a str, &'a str, &'a str) {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    match text.split_once(open.as_str()) {
        Some((before, rest)) => match rest.split_once(close.as_str()) {
            Some((inner, after)) => (before, inner, after),
            None => (before, rest, ""),
        },
        None => (text, "", ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_splits_a_sentence() {
        let text = "Read <link>the guide</link> first.";
        assert_eq!(split_tag(text, "link"), ("Read ", "the guide", " first."));
    }

    #[test]
    fn a_sentence_without_the_tag_comes_back_whole() {
        assert_eq!(split_tag("No link here.", "link"), ("No link here.", "", ""));
    }

    #[test]
    fn english_is_the_first_and_default_language() {
        assert_eq!(Lang::ALL.first(), Some(&Lang::En));
        assert_eq!(Lang::default(), Lang::En);
        assert_eq!(Lang::En.tag(), "en");
    }
}
