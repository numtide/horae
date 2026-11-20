//! Parsing and checks of the Fluent catalogs, shared by `build.rs` and
//! `tests/build_checks.rs`. Every check returns an `Err` naming the file and the
//! id; `build.rs` turns it into a failed build.

use std::collections::{BTreeMap, BTreeSet};

use fluent_syntax::ast::{Entry, Expression, InlineExpression, Pattern, PatternElement};

/// One message of a catalog, as the generator needs it.
#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    /// The variables it takes, directly or through the messages it references.
    pub vars: BTreeSet<String>,
    /// Its text with placeables as written (`Token “{ $name }” created`), for the
    /// generated method's doc comment.
    pub text: String,
}

/// Message id → message.
pub type Catalog = BTreeMap<String, Message>;

/// The Rust name of a message's method or of a variable's parameter.
pub fn rust_name(id: &str) -> String {
    id.replace('-', "_")
}

/// Strict and reserved keywords of Rust 2024, which cannot name a parameter or a method.
const KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl",
    "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "try", "type",
    "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// The methods `Tr` defines in `src/lib.rs`, which a generated method cannot repeat.
const TR_METHODS: &[&str] = &["new", "lang", "format"];

/// Direct variables and direct references of one entry.
#[derive(Default)]
struct Found {
    vars: BTreeSet<String>,
    messages: BTreeSet<String>,
    terms: BTreeSet<String>,
}

/// Parses one catalog (`file` names it in errors) and checks it on its own: syntax,
/// duplicate ids, messages without a value, references to unknown messages or
/// terms, two ids with one method name, and keywords as ids or variables.
pub fn parse(file: &str, source: &str) -> Result<Catalog, String> {
    let resource = fluent_syntax::parser::parse(source).map_err(|(_, errors)| {
        let first = &errors[0];
        let line = source[..first.pos.start].lines().count().max(1);
        format!(
            "{file}:{line}: {} syntax errors, first: {:?}",
            errors.len(),
            first.kind
        )
    })?;

    let mut messages: BTreeMap<String, (Found, String)> = BTreeMap::new();
    let mut terms: BTreeMap<String, Found> = BTreeMap::new();
    for entry in &resource.body {
        match entry {
            Entry::Message(message) => {
                let id = message.id.name.to_string();
                let Some(value) = &message.value else {
                    return Err(format!("{file}: `{id}` has no value"));
                };
                let mut found = Found::default();
                pattern(value, &mut found);
                if messages.insert(id.clone(), (found, text(value))).is_some() {
                    return Err(format!("{file}: `{id}` is defined twice"));
                }
            }
            Entry::Term(term) => {
                let id = term.id.name.to_string();
                let mut found = Found::default();
                pattern(&term.value, &mut found);
                if terms.insert(id.clone(), found).is_some() {
                    return Err(format!("{file}: `-{id}` is defined twice"));
                }
            }
            _ => {}
        }
    }

    for (id, found) in messages
        .iter()
        .map(|(id, (found, _))| (id.as_str(), found))
        .chain(terms.iter().map(|(id, found)| (id.as_str(), found)))
    {
        if let Some(missing) = found.messages.iter().find(|m| !messages.contains_key(*m)) {
            return Err(format!(
                "{file}: `{id}` references unknown message `{missing}`"
            ));
        }
        if let Some(missing) = found.terms.iter().find(|t| !terms.contains_key(*t)) {
            return Err(format!(
                "{file}: `{id}` references unknown term `-{missing}`"
            ));
        }
    }

    let mut methods: BTreeMap<String, &str> = BTreeMap::new();
    for (id, (found, _)) in &messages {
        let method = rust_name(id);
        if KEYWORDS.contains(&method.as_str()) {
            return Err(format!(
                "{file}: `{id}` is a Rust keyword, so it cannot name a method"
            ));
        }
        if TR_METHODS.contains(&method.as_str()) {
            return Err(format!(
                "{file}: `{id}` would become `Tr::{method}`, which `Tr` already has; rename it"
            ));
        }
        if let Some(other) = methods.insert(method.clone(), id) {
            return Err(format!(
                "{file}: `{other}` and `{id}` both become the method `{method}`"
            ));
        }
        if let Some(var) = found
            .vars
            .iter()
            .find(|v| KEYWORDS.contains(&rust_name(v).as_str()))
        {
            return Err(format!(
                "{file}: `{id}` uses the variable `${var}`, a Rust keyword; rename it"
            ));
        }
    }

    Ok(messages
        .keys()
        .map(|id| {
            let mut vars = BTreeSet::new();
            resolve(id, &messages, &mut vars, &mut BTreeSet::new());
            let text = messages[id].1.clone();
            (id.clone(), Message { vars, text })
        })
        .collect())
}

/// Checks a translation against English: it fails on a message English lacks or
/// on other variables than English's, and returns how many English messages it
/// lacks (those show in English).
pub fn check_translation(
    file: &str,
    english: &Catalog,
    translated: &Catalog,
) -> Result<usize, String> {
    for (id, message) in translated {
        match english.get(id) {
            None => return Err(format!("{file}: `{id}` is not in the English catalog")),
            Some(source) if source.vars != message.vars => {
                return Err(format!(
                    "{file}: `{id}` uses {:?}, English uses {:?}",
                    message.vars, source.vars
                ));
            }
            Some(_) => {}
        }
    }
    Ok(english
        .keys()
        .filter(|id| !translated.contains_key(*id))
        .count())
}

/// Adds the variables of `id` and of every message it references, once each.
fn resolve(
    id: &str,
    messages: &BTreeMap<String, (Found, String)>,
    vars: &mut BTreeSet<String>,
    seen: &mut BTreeSet<String>,
) {
    if !seen.insert(id.to_string()) {
        return;
    }
    let (found, _) = &messages[id];
    vars.extend(found.vars.iter().cloned());
    for reference in &found.messages {
        resolve(reference, messages, vars, seen);
    }
}

fn pattern(pattern: &Pattern<&str>, found: &mut Found) {
    for element in &pattern.elements {
        if let PatternElement::Placeable { expression } = element {
            expr(expression, found);
        }
    }
}

fn expr(expression: &Expression<&str>, found: &mut Found) {
    match expression {
        Expression::Select { selector, variants } => {
            inline(selector, found);
            for variant in variants {
                pattern(&variant.value, found);
            }
        }
        Expression::Inline(inline_expr) => inline(inline_expr, found),
    }
}

fn inline(expression: &InlineExpression<&str>, found: &mut Found) {
    match expression {
        InlineExpression::VariableReference { id } => {
            found.vars.insert(id.name.to_string());
        }
        InlineExpression::MessageReference { id, .. } => {
            found.messages.insert(id.name.to_string());
        }
        // A term's arguments are its own, not the message's.
        InlineExpression::TermReference { id, .. } => {
            found.terms.insert(id.name.to_string());
        }
        InlineExpression::FunctionReference { arguments, .. } => {
            for argument in &arguments.positional {
                inline(argument, found);
            }
            for argument in &arguments.named {
                inline(&argument.value, found);
            }
        }
        InlineExpression::Placeable { expression } => expr(expression, found),
        InlineExpression::StringLiteral { .. } | InlineExpression::NumberLiteral { .. } => {}
    }
}

/// The pattern as a reader of the code wants it: text as is, placeables as
/// written, and a selector by its default variant.
fn text(pattern: &Pattern<&str>) -> String {
    let mut out = String::new();
    for element in &pattern.elements {
        match element {
            PatternElement::TextElement { value } => out.push_str(value),
            PatternElement::Placeable { expression } => match expression {
                Expression::Select { variants, .. } => {
                    if let Some(default) = variants.iter().find(|v| v.default) {
                        out.push_str(&text(&default.value));
                    }
                }
                // `{"{"}`, the escape for a literal brace.
                Expression::Inline(InlineExpression::StringLiteral { value }) => {
                    out.push_str(value);
                }
                Expression::Inline(inline) => {
                    out.push_str("{ ");
                    out.push_str(&inline_text(inline));
                    out.push_str(" }");
                }
            },
        }
    }
    out
}

fn inline_text(expression: &InlineExpression<&str>) -> String {
    match expression {
        InlineExpression::VariableReference { id } => format!("${}", id.name),
        InlineExpression::MessageReference { id, .. } => id.name.to_string(),
        InlineExpression::TermReference { id, .. } => format!("-{}", id.name),
        InlineExpression::FunctionReference { id, .. } => format!("{}(…)", id.name),
        InlineExpression::StringLiteral { value } => format!("\"{value}\""),
        InlineExpression::NumberLiteral { value } => value.to_string(),
        InlineExpression::Placeable { .. } => "…".to_string(),
    }
}
