//! Every English message is used: its method is called somewhere under `crates/`
//! outside this crate, or a used message references it. `dead_code` cannot see
//! this, because the generated methods are public. A call from a test (`crates/horae/tests`)
//! counts as a use too.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use fluent_syntax::ast::{Entry, Expression, InlineExpression, Pattern, PatternElement};

#[test]
fn every_english_message_is_used() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(manifest.join("locales/en/horae.ftl")).unwrap();
    let resource = fluent_syntax::parser::parse(source.as_str()).expect("build.rs checked it");

    // Message id → the messages it references.
    let mut references: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for entry in &resource.body {
        if let Entry::Message(message) = entry {
            let mut found = BTreeSet::new();
            if let Some(value) = &message.value {
                pattern(value, &mut found);
            }
            references.insert(message.id.name.to_string(), found);
        }
    }

    let mut code = String::new();
    let mut files = Vec::new();
    rust_files(&manifest.join(".."), manifest, &mut files);
    for file in files {
        code.push_str(&std::fs::read_to_string(file).unwrap());
    }

    let mut used: BTreeSet<&str> = references
        .keys()
        .filter(|id| code.contains(&format!(".{}(", id.replace('-', "_"))))
        .map(String::as_str)
        .collect();
    let mut pending: Vec<&str> = used.iter().copied().collect();
    while let Some(id) = pending.pop() {
        for reference in &references[id] {
            if used.insert(reference) {
                pending.push(reference);
            }
        }
    }

    let unused: Vec<&str> = references
        .keys()
        .map(String::as_str)
        .filter(|id| !used.contains(id))
        .collect();
    assert!(
        unused.is_empty(),
        "messages in locales/en/horae.ftl that no code uses; delete them: {unused:?}"
    );
}

/// Every `.rs` file under `dir`, skipping `skip` (this crate) and build output.
fn rust_files(dir: &Path, skip: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            let skipped = path.canonicalize().ok() == skip.canonicalize().ok()
                || path.file_name().is_some_and(|n| n == "target");
            if !skipped {
                rust_files(&path, skip, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn pattern(pattern: &Pattern<&str>, found: &mut BTreeSet<String>) {
    for element in &pattern.elements {
        if let PatternElement::Placeable { expression } = element {
            expr(expression, found);
        }
    }
}

fn expr(expression: &Expression<&str>, found: &mut BTreeSet<String>) {
    match expression {
        Expression::Select { selector, variants } => {
            inline(selector, found);
            for variant in variants {
                pattern(&variant.value, found);
            }
        }
        Expression::Inline(expression) => inline(expression, found),
    }
}

fn inline(expression: &InlineExpression<&str>, found: &mut BTreeSet<String>) {
    match expression {
        InlineExpression::MessageReference { id, .. } => {
            found.insert(id.name.to_string());
        }
        InlineExpression::Placeable { expression } => expr(expression, found),
        InlineExpression::FunctionReference { arguments, .. } => {
            for argument in &arguments.positional {
                inline(argument, found);
            }
        }
        _ => {}
    }
}
