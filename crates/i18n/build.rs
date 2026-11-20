//! Reads the Fluent catalogs in `locales/<lang>/horae.ftl` and generates:
//!
//! - `Lang`, one variant per locale directory, English first;
//! - one method on `Tr` per English message, named after its id
//!   (`api-token-created` → `api_token_created`), whose parameters are the
//!   message's variables, so a missing key or argument fails to compile. Its doc
//!   comment is the English text.
//!
//! English is the source catalog. A translation may lack messages (they fall
//! back to English and are reported as warnings), but it fails the build when it
//! has a message English does not, or uses other variables than English does.
//! The checks live in `build/checks.rs`, which `tests/build_checks.rs` also runs.

use std::fmt::Write as _;
use std::path::Path;

#[path = "build/checks.rs"]
mod checks;

use checks::{Catalog, rust_name};

const SOURCE: &str = "en";

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    println!("cargo:rerun-if-changed={}", root.display());
    println!("cargo:rerun-if-changed=build/checks.rs");

    let mut locales: Vec<String> = std::fs::read_dir(&root)
        .expect("crates/i18n/locales exists")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("horae.ftl").is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    locales.sort_by_key(|lang| (lang != SOURCE, lang.clone()));
    assert_eq!(
        locales.first().map(String::as_str),
        Some(SOURCE),
        "locales/en/horae.ftl is required"
    );

    let catalogs: Vec<Catalog> = locales
        .iter()
        .map(|lang| {
            let path = root.join(lang).join("horae.ftl");
            println!("cargo:rerun-if-changed={}", path.display());
            let source = std::fs::read_to_string(&path).expect("readable catalog");
            checks::parse(&format!("locales/{lang}/horae.ftl"), &source)
                .unwrap_or_else(|e| panic!("{e}"))
        })
        .collect();

    let english = &catalogs[0];
    for (lang, translated) in locales.iter().zip(&catalogs).skip(1) {
        let file = format!("locales/{lang}/horae.ftl");
        let missing = checks::check_translation(&file, english, translated)
            .unwrap_or_else(|e| panic!("{e}"));
        if missing > 0 {
            println!("cargo:warning={file} lacks {missing} messages; they show in English");
        }
    }

    let mut out = String::new();
    out.push_str("/// A language Horae has a catalog for.\n");
    out.push_str("#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]\npub enum Lang {\n");
    for (i, lang) in locales.iter().enumerate() {
        if i == 0 {
            out.push_str("    #[default]\n");
        }
        writeln!(out, "    {},", variant(lang)).unwrap();
    }
    out.push_str("}\n\nimpl Lang {\n    pub const ALL: &[Lang] = &[");
    for lang in &locales {
        write!(out, "Lang::{}, ", variant(lang)).unwrap();
    }
    out.push_str("];\n\n    /// The BCP 47 tag, as in `<html lang>`.\n    pub fn tag(self) -> &'static str {\n        match self {\n");
    for lang in &locales {
        writeln!(out, "            Lang::{} => {lang:?},", variant(lang)).unwrap();
    }
    out.push_str("        }\n    }\n}\n\n");
    out.push_str(
        "/// Each catalog's source, in the order of [`Lang::ALL`].\nconst SOURCES: &[&str] = &[\n",
    );
    for lang in &locales {
        let path = root.join(lang).join("horae.ftl");
        writeln!(out, "    include_str!({:?}),", path.display().to_string()).unwrap();
    }
    out.push_str("];\n\nimpl Tr {\n");
    for (id, message) in english {
        for line in message.text.lines() {
            writeln!(out, "    #[doc = {:?}]", format!(" {line}")).unwrap();
        }
        let method = rust_name(id);
        if message.vars.is_empty() {
            writeln!(out, "    pub fn {method}(&self) -> String {{\n        self.format({id:?}, None)\n    }}\n").unwrap();
            continue;
        }
        let params: Vec<String> = message
            .vars
            .iter()
            .map(|var| format!("{}: impl Into<FluentValue<'a>>", rust_name(var)))
            .collect();
        writeln!(
            out,
            "    pub fn {method}<'a>(&self, {}) -> String {{",
            params.join(", ")
        )
        .unwrap();
        out.push_str("        let mut args = FluentArgs::new();\n");
        for var in &message.vars {
            writeln!(out, "        args.set({var:?}, {});", rust_name(var)).unwrap();
        }
        writeln!(out, "        self.format({id:?}, Some(&args))\n    }}\n").unwrap();
    }
    out.push_str("}\n");

    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("messages.rs");
    std::fs::write(dest, out).unwrap();
}

fn variant(lang: &str) -> String {
    lang.split(['-', '_'])
        .map(|part| {
            let mut chars = part.chars();
            let first = chars.next().unwrap().to_ascii_uppercase();
            first.to_string() + &chars.as_str().to_ascii_lowercase()
        })
        .collect()
}
