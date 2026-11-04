//! Ratcheted checks for the rules in `.agents/skills/dioxus-best-practices/SKILL.md`.
//!
//! `rsx!` bodies are parsed with `dioxus-rsx`, the parser the `rsx!` macro itself uses,
//! and the Rust around them with `syn`. Each rule has a count in `baseline.toml`; the
//! check fails when a count moves either way, so new violations are caught and fixed
//! ones lower the baseline.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use dioxus_rsx::{
    AttributeName, AttributeValue, BodyNode, CallBody, ElementName, HotLiteral, IfChain,
};
use proc_macro2::Span;
use syn::visit::{self, Visit};
use syn::{BinOp, Expr, ReturnType, Stmt, Type};

struct Rule {
    id: &'static str,
    /// The `## ` heading in SKILL.md this rule enforces.
    section: &'static str,
    /// The Dioxus 0.7.9 file (relative to `packages/`) and symbol in `references/source-evidence.md`.
    evidence_file: &'static str,
    evidence_symbol: &'static str,
    why: &'static str,
    fix: &'static str,
    /// Enforced by `disallowed-methods` in `clippy.toml` rather than by this checker.
    clippy: bool,
}

const RULES: &[Rule] = &[
    Rule {
        id: "dioxus/inline-callback-prop",
        section: "Props: what compares equal",
        evidence_file: "core/src/events.rs",
        evidence_symbol: "impl PartialEq for Callback",
        why: "an inline closure is a new Callback on every parent render, so the child's props never compare equal and it re-renders every time",
        fix: "hoist it into `let on_x = use_callback(move |v| …);` and pass `on_x`",
        clippy: false,
    },
    Rule {
        id: "dioxus/callback-new-in-render",
        section: "Props: what compares equal",
        evidence_file: "core/src/events.rs",
        evidence_symbol: "Callback::new",
        why: "it allocates a new owned callback on every render, kept until the component unmounts, and it never compares equal",
        fix: "use `use_callback`",
        clippy: true,
    },
    Rule {
        id: "dioxus/resource-gate",
        section: "Resources and loading",
        evidence_file: "hooks/src/use_resource.rs",
        evidence_symbol: "restart",
        why: "restart() sets Pending but keeps the old value, so gating on state() != Ready unmounts the subtree and drops its local state on every reload",
        fix: "gate only the first load with `r.read().is_none()`, and show the stale data with `aria-busy` while a reload runs",
        clippy: false,
    },
    Rule {
        id: "dioxus/render-helper",
        section: "Subscriptions",
        evidence_file: "signals/src",
        evidence_symbol: "reads subscribe the running scope",
        why: "a plain fn's signal reads subscribe the component that calls it, so every change re-renders the whole caller",
        fix: "make the helper a `#[component]` so its reads subscribe only itself",
        clippy: false,
    },
    Rule {
        id: "dioxus/internal-href",
        section: "Events, links, effects",
        evidence_file: "router/src/components/link.rs",
        evidence_symbol: "Link",
        why: "a plain `a { href }` is a full page load, which restarts the WASM app and loses the shared session",
        fix: "use `Link { to: Route::… }`",
        clippy: false,
    },
];

/// Paths served by Axum routes rather than the Dioxus router, where a full navigation is intended.
const SERVER_PATHS: &[&str] = &["/api/", "/auth/", "/harvest/", "/health"];

#[derive(Debug, PartialEq)]
struct Finding {
    rule: &'static str,
    line: usize,
}

#[derive(Default)]
struct Checker {
    findings: Vec<Finding>,
    /// Inside a fn that returns `Element`.
    in_render: bool,
    /// Closures between the current point and the enclosing fn body.
    closure_depth: usize,
    /// Locals of the current fn bound to a resource state comparison, such as
    /// `let ready = r.state()() == UseResourceState::Ready;`, with whether they mean "not ready".
    state_flags: Vec<(String, bool)>,
}

impl Checker {
    /// `r.state()() != UseResourceState::Ready`, a local bound to it, or `!` a local bound to
    /// `== Ready`, alone or joined by `||`. An `&&` narrows the gate (for example to the
    /// first load), so it is not flagged.
    fn is_not_ready(&self, cond: &Expr) -> bool {
        match cond {
            Expr::Paren(p) => self.is_not_ready(&p.expr),
            Expr::Binary(b) if matches!(b.op, BinOp::Or(_)) => {
                self.is_not_ready(&b.left) || self.is_not_ready(&b.right)
            }
            Expr::Unary(u) if matches!(u.op, syn::UnOp::Not(_)) => {
                self.flag(&u.expr) == Some(false)
            }
            _ => state_comparison(cond) == Some(true) || self.flag(cond) == Some(true),
        }
    }

    fn flag(&self, expr: &Expr) -> Option<bool> {
        let Expr::Path(p) = expr else { return None };
        let name = p.path.get_ident()?.to_string();
        self.state_flags
            .iter()
            .rev()
            .find(|(n, _)| *n == name)
            .map(|(_, not_ready)| *not_ready)
    }

    fn push(&mut self, rule: &'static str, span: Span) {
        self.findings.push(Finding {
            rule,
            line: span.start().line,
        });
    }

    fn nodes(&mut self, nodes: &[BodyNode]) {
        for node in nodes {
            match node {
                BodyNode::Element(el) => {
                    if is_internal_link(el) {
                        self.push("dioxus/internal-href", el.name.span());
                    }
                    self.attributes(&el.raw_attributes);
                    self.nodes(&el.children);
                }
                BodyNode::Component(component) => {
                    for field in &component.fields {
                        if let AttributeName::BuiltIn(name) = &field.name
                            && name.to_string().starts_with("on")
                            && is_closure(&field.value)
                        {
                            self.push("dioxus/inline-callback-prop", name.span());
                        }
                    }
                    self.attributes(&component.fields);
                    self.nodes(&component.children.roots);
                }
                BodyNode::RawExpr(raw) => {
                    if let Ok(expr) = raw.expr.as_expr() {
                        self.visit_expr(&expr);
                    }
                }
                BodyNode::ForLoop(lp) => {
                    self.visit_expr(&lp.expr);
                    self.nodes(&lp.body.roots);
                }
                BodyNode::IfChain(chain) => self.if_chain(chain),
                BodyNode::Text(_) => {}
            }
        }
    }

    fn if_chain(&mut self, chain: &IfChain) {
        // `if !ready { loading } else { content }` unmounts `content` just as an early return does.
        let swaps = chain.else_branch.is_some() || chain.else_if_branch.is_some();
        if swaps && self.is_not_ready(&chain.cond) {
            self.push("dioxus/resource-gate", chain.if_token.span);
        }
        self.visit_expr(&chain.cond);
        self.nodes(&chain.then_branch.roots);
        if let Some(next) = &chain.else_if_branch {
            self.if_chain(next);
        }
        if let Some(body) = &chain.else_branch {
            self.nodes(&body.roots);
        }
    }

    fn attributes(&mut self, attributes: &[dioxus_rsx::Attribute]) {
        for attribute in attributes {
            match &attribute.value {
                AttributeValue::EventTokens(closure) => {
                    if let Ok(body) = closure.body.as_expr() {
                        self.closure_depth += 1;
                        self.visit_expr(&body);
                        self.closure_depth -= 1;
                    }
                }
                AttributeValue::AttrExpr(expr) => {
                    if let Ok(expr) = expr.as_expr() {
                        self.visit_expr(&expr);
                    }
                }
                AttributeValue::IfExpr(value) => self.visit_expr_if(&value.if_expr),
                AttributeValue::Shorthand(_) | AttributeValue::AttrLiteral(_) => {}
            }
        }
    }
}

impl<'ast> Visit<'ast> for Checker {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let render = returns_element(&item.sig.output);
        let component = item.attrs.iter().any(|a| a.path().is_ident("component"));
        if render && !component && item.sig.ident.to_string().starts_with("render_") {
            self.push("dioxus/render-helper", item.sig.ident.span());
        }
        let saved = (
            self.in_render,
            self.closure_depth,
            std::mem::take(&mut self.state_flags),
        );
        (self.in_render, self.closure_depth) = (render, 0);
        visit::visit_item_fn(self, item);
        (self.in_render, self.closure_depth, self.state_flags) = saved;
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if let syn::Pat::Ident(name) = &local.pat
            && let Some(init) = &local.init
            && let Some(not_ready) = state_comparison(&init.expr)
        {
            self.state_flags.push((name.ident.to_string(), not_ready));
        }
        visit::visit_local(self, local);
    }

    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        self.closure_depth += 1;
        visit::visit_expr_closure(self, closure);
        self.closure_depth -= 1;
    }

    fn visit_expr_if(&mut self, expr: &'ast syn::ExprIf) {
        if self.in_render
            && self.closure_depth == 0
            && self.is_not_ready(&expr.cond)
            && returns_rsx(&expr.then_branch)
        {
            self.push("dioxus/resource-gate", expr.if_token.span);
        }
        visit::visit_expr_if(self, expr);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac.path.is_ident("rsx")
            && let Ok(body) = syn::parse2::<CallBody>(mac.tokens.clone())
        {
            self.nodes(&body.body.roots);
        }
    }
}

fn returns_element(output: &ReturnType) -> bool {
    matches!(output, ReturnType::Type(_, ty)
        if matches!(&**ty, Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == "Element")))
}

fn returns_rsx(block: &syn::Block) -> bool {
    block.stmts.iter().any(|stmt| {
        matches!(stmt, Stmt::Expr(Expr::Return(ret), _)
            if matches!(ret.expr.as_deref(), Some(Expr::Macro(m)) if m.mac.path.is_ident("rsx")))
    })
}

/// `Some(true)` for `r.state()() != UseResourceState::Ready`, `Some(false)` for `==`.
fn state_comparison(expr: &Expr) -> Option<bool> {
    let Expr::Binary(b) = expr else { return None };
    let not_ready = match b.op {
        BinOp::Ne(_) => true,
        BinOp::Eq(_) => false,
        _ => return None,
    };
    let compares = (is_state_call(&b.left) && is_ready(&b.right))
        || (is_ready(&b.left) && is_state_call(&b.right));
    compares.then_some(not_ready)
}

fn is_state_call(expr: &Expr) -> bool {
    match expr {
        Expr::MethodCall(m) => m.method == "state",
        Expr::Call(c) => is_state_call(&c.func),
        _ => false,
    }
}

fn is_ready(expr: &Expr) -> bool {
    let Expr::Path(p) = expr else { return false };
    let names: Vec<_> = p
        .path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    names.ends_with(&["UseResourceState".into(), "Ready".into()])
}

fn is_closure(value: &AttributeValue) -> bool {
    match value {
        AttributeValue::EventTokens(_) => true,
        AttributeValue::AttrExpr(expr) => matches!(expr.as_expr(), Ok(Expr::Closure(_))),
        _ => false,
    }
}

fn is_internal_link(el: &dioxus_rsx::Element) -> bool {
    if !matches!(&el.name, ElementName::Ident(name) if name == "a") {
        return false;
    }
    let attr = |wanted: &str| {
        el.raw_attributes
            .iter()
            .find(|a| matches!(&a.name, AttributeName::BuiltIn(n) if n == wanted))
    };
    // A new tab is a full navigation by design; `Link` itself skips routing for it.
    if attr("target").is_some() {
        return false;
    }
    let Some(href) = attr("href") else {
        return false;
    };
    let AttributeValue::AttrLiteral(HotLiteral::Fmted(href)) = &href.value else {
        return false;
    };
    let href = href.source.value();
    href.starts_with('/') && !SERVER_PATHS.iter().any(|p| href.starts_with(p))
}

fn check_source(src: &str) -> syn::Result<Vec<Finding>> {
    let file = syn::parse_file(src)?;
    let mut checker = Checker::default();
    checker.visit_file(&file);
    Ok(checker.findings)
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Allowance {
    count: usize,
    reason: String,
}

fn load_baseline(path: &Path) -> Result<BTreeMap<String, Allowance>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let baseline: BTreeMap<String, Allowance> =
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    for (id, allowance) in &baseline {
        if !RULES.iter().any(|r| r.id == id && !r.clippy) {
            return Err(format!(
                "{}: `{id}` is not a rule this checker runs",
                path.display()
            ));
        }
        if allowance.reason.trim().is_empty() {
            return Err(format!("{}: `{id}` needs a reason", path.display()));
        }
    }
    Ok(baseline)
}

fn main() -> ExitCode {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let baseline_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("baseline.toml");
    let baseline = match load_baseline(&baseline_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let mut files = Vec::new();
    if let Err(e) = rust_files(&root.join("crates/horae/src"), &mut files) {
        eprintln!("reading crates/horae/src: {e}");
        return ExitCode::FAILURE;
    }
    files.sort();

    let mut found: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for path in &files {
        let shown = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .display()
            .to_string();
        let src = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{shown}: {e}");
                return ExitCode::FAILURE;
            }
        };
        match check_source(&src) {
            Ok(findings) => {
                for f in findings {
                    found
                        .entry(f.rule)
                        .or_default()
                        .push(format!("{shown}:{}", f.line));
                }
            }
            Err(e) => {
                eprintln!("{shown}: does not parse: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    let mut ok = true;
    for rule in RULES.iter().filter(|r| !r.clippy) {
        let sites = found.get(rule.id).map(Vec::as_slice).unwrap_or_default();
        let allowed = baseline.get(rule.id).map_or(0, |a| a.count);
        if sites.len() > allowed {
            ok = false;
            eprintln!(
                "{}: {} found, baseline allows {allowed}. One of these is new:",
                rule.id,
                sites.len()
            );
            for site in sites {
                eprintln!("  {site}: {}", rule.id);
            }
            eprintln!("  why: {}", rule.why);
            eprintln!("  fix: {}", rule.fix);
            eprintln!(
                "  see: .agents/skills/dioxus-best-practices/SKILL.md \"{}\" (dioxus 0.7.9 {} `{}`)\n",
                rule.section, rule.evidence_file, rule.evidence_symbol
            );
        } else if sites.len() < allowed {
            ok = false;
            eprintln!(
                "{}: {} found, baseline allows {allowed}. Lower `count` in crates/dioxus-lint/baseline.toml to {}.",
                rule.id,
                sites.len(),
                sites.len()
            );
        } else {
            println!("{}: {} (baseline {allowed})", rule.id, sites.len());
        }
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// The nix check passes the skill in explicitly, because the package source omits `.md` files.
    fn skill_dir() -> PathBuf {
        std::env::var_os("DIOXUS_SKILL_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest_dir().join("../../.agents/skills/dioxus-best-practices"))
    }

    #[test]
    fn fixtures_flag_their_rule_and_nothing_else() {
        for rule in RULES.iter().filter(|r| !r.clippy) {
            let dir = manifest_dir()
                .join("fixtures")
                .join(rule.id.trim_start_matches("dioxus/"));
            let bad = std::fs::read_to_string(dir.join("bad.rs")).unwrap();
            let good = std::fs::read_to_string(dir.join("good.rs")).unwrap();
            // A marker comment sits on its own line, where rustfmt leaves it, above the flagged line.
            let marked: Vec<_> = (bad.lines().enumerate())
                .filter(|(_, l)| l.trim() == "// next line flagged")
                .map(|(i, _)| Finding {
                    rule: rule.id,
                    line: i + 2,
                })
                .collect();
            assert!(
                !marked.is_empty(),
                "{}: bad.rs has no `// next line flagged` marker",
                rule.id
            );
            assert_eq!(check_source(&bad).unwrap(), marked, "{}: bad.rs", rule.id);
            let good = check_source(&good).unwrap();
            assert_eq!(good, vec![], "{}: good.rs is flagged", rule.id);
        }
    }

    #[test]
    fn every_rule_names_a_skill_heading_and_evidence() {
        let skill = std::fs::read_to_string(skill_dir().join("SKILL.md")).unwrap();
        let evidence =
            std::fs::read_to_string(skill_dir().join("references/source-evidence.md")).unwrap();
        for rule in RULES {
            assert!(
                skill
                    .lines()
                    .any(|l| l.starts_with(&format!("## {}", rule.section))),
                "{}: SKILL.md has no heading \"## {}\"",
                rule.id,
                rule.section
            );
            assert!(
                evidence.contains(rule.evidence_file),
                "{}: source-evidence.md does not mention {}",
                rule.id,
                rule.evidence_file
            );
        }
    }

    #[test]
    fn clippy_rules_are_configured() {
        let clippy = std::fs::read_to_string(manifest_dir().join("../../clippy.toml")).unwrap();
        for rule in RULES.iter().filter(|r| r.clippy) {
            assert!(
                clippy.contains(rule.id),
                "{} is missing from clippy.toml",
                rule.id
            );
        }
    }

    #[test]
    fn baseline_entries_are_known_rules_with_reasons() {
        load_baseline(&manifest_dir().join("baseline.toml")).unwrap();
    }
}
