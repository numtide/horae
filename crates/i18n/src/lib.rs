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

/// Splits a message into its plain runs and its `<tag>…</tag>` runs, in order,
/// for a sentence with several tagged parts: `[(Some("actor"), "Ana"), (None,
/// " made "), …]`. A `<` that opens no complete tag stays text.
///
/// Format the message with placeholders for the values people typed, split
/// it, then put the values back: a name containing `<b>` or `</actor>` is then
/// never read as markup.
pub fn split_tags(text: &str) -> Vec<(Option<&str>, &str)> {
    let mut parts = Vec::new();
    let mut rest = text;
    let mut plain = 0;
    while let Some(open) = rest[plain..].find('<').map(|at| plain + at) {
        let tagged = rest[open + 1..].split_once('>').and_then(|(tag, after)| {
            let well_formed = !tag.is_empty() && tag.chars().all(|c| c.is_ascii_lowercase());
            let (inner, tail) = after.split_once(&format!("</{tag}>"))?;
            well_formed.then_some((tag, inner, tail))
        });
        let Some((tag, inner, tail)) = tagged else {
            plain = open + 1;
            continue;
        };
        if open > 0 {
            parts.push((None, &rest[..open]));
        }
        parts.push((Some(tag), inner));
        rest = tail;
        plain = 0;
    }
    if !rest.is_empty() {
        parts.push((None, rest));
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_split_a_sentence_into_runs() {
        assert_eq!(
            split_tags("<actor>Ana</actor> made <person>Bo</person> a manager"),
            vec![
                (Some("actor"), "Ana"),
                (None, " made "),
                (Some("person"), "Bo"),
                (None, " a manager"),
            ]
        );
    }

    #[test]
    fn a_stray_angle_bracket_stays_text() {
        assert_eq!(
            split_tags("1 < 2 and <actor>Ana</actor>"),
            vec![(None, "1 < 2 and "), (Some("actor"), "Ana")]
        );
        assert_eq!(split_tags("no tags < here"), vec![(None, "no tags < here")]);
        assert_eq!(split_tags("<actor>open"), vec![(None, "<actor>open")]);
    }

    #[test]
    fn a_tag_splits_a_sentence() {
        let text = "Read <link>the guide</link> first.";
        assert_eq!(split_tag(text, "link"), ("Read ", "the guide", " first."));
    }

    #[test]
    fn a_sentence_without_the_tag_comes_back_whole() {
        assert_eq!(
            split_tag("No link here.", "link"),
            ("No link here.", "", "")
        );
    }

    #[test]
    fn english_is_the_first_and_default_language() {
        assert_eq!(Lang::ALL.first(), Some(&Lang::En));
        assert_eq!(Lang::default(), Lang::En);
        assert_eq!(Lang::En.tag(), "en");
    }
}
