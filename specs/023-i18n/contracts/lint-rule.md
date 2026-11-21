# Contract: lint rule `dioxus/literal-text`

The sixth rule of `crates/dioxus-lint` (FR-020 to FR-023, C-7). Its counting logic is the
proof of concept's `crates/dioxus-lint/src/bin/ui-strings.rs` (`study/i18n`, `26e562bf`); the
binary itself is not merged.

## Entry in `RULES`

| Field | Value |
| --- | --- |
| `id` | `dioxus/literal-text` |
| `section` | `Interface text` (a new `## Interface text` heading in `.agents/skills/dioxus-best-practices/SKILL.md`) |
| `evidence_file` | `rsx/src/text_node.rs` (the `dioxus-rsx` 0.7.9 parser of text nodes, with an excerpt added to `references/source-evidence.md`) |
| `why` | text written in `rsx!` cannot be translated and escapes the catalog, so each language would mean editing the screen |
| `fix` | move the text into `crates/i18n/locales/en/horae.ftl` and call `tr.<id>()` (see the skill's *Interface text*) |
| `clippy` | `false` |

## What it counts

Per file of `crates/horae/src`, the same files the other rules walk:

| Kind | Counted when | Example counted | Not counted |
| --- | --- | --- | --- |
| Text node | a literal segment has a letter | `"Save"`, `"v{plugin.version}"`, `"{n} hours"` | `"{plugin.name}"`, `"·"`, `"%"` |
| Readable attribute | its value is a literal (formatted or not) and its name is one of `placeholder`, `title`, `alt`, `label`, `aria_label`, `aria_description`, `aria_valuetext`, `aria_roledescription` (and their dashed spellings), `data-tooltip` | `aria_label: "Task pages"` | `aria_label: tr.tasks_pages()` |
| Prose prop | a component prop whose value is a literal with a letter and whose name is not a machine prop | `label: "Name"`, `title: "No projects yet"` | `class: "btn"`, `id: "tasks-new"`, `to: …` |

Machine props are the proof of concept's `MACHINE_PROPS` list (`class`, `trigger_class`, `id`,
`key`, `href`, `to`, `name`, `icon`, `r#type`, `variant`, `kind`, `tone`, `size`, `width`,
`autocomplete`, `inputmode`, `pattern`, `accept`, `form`, `role`, `testid`, `test_id`, `target`,
`rel`, `style`, `min`, `max`, `step`, `value`, `field`, `input_id`, `list_id`, `menu_id`,
`popover_id`, `panel_id`), plus the ones I-1 found when taking the baseline, each with its reason
in the code: the element ids `labelledby`, `focus_fallback`, `title_id`, `retry_id` and
`focus_on_recovery`, the test hook `data-testid`, `MetricCard`'s `direction` and
`document::Link`'s `sizes`. A design PR that adds a component with another machine prop adds it to
the list in the same PR.

Text inside `if`/`match` arms and `for` bodies of `rsx!` counts the same as at top level, and so
does a text node written as an expression (`{"Save"}`) and each literal branch of an attribute or
prop written with `if` (`placeholder: if x { "Add tags…" } else { "" }`). Test code
(`#[cfg(test)]` and `#[cfg(all(test, …))]` items, `#[test]` functions, `tests.rs`, `*_tests.rs`)
is not counted;
`#[cfg(not(test))]` and `#[cfg(feature = "test-support")]` are.

### Known gaps

The rule counts literals only. These sites hold interface text it does not see; the info count
includes them, and each migration PR (I-2 to I-12) looks for them in its area by hand:

- a readable attribute or prose prop built with an expression: `title: format!("Delete {name}")`,
  `label: name.to_string()`, a `match`;
- `value: "Save changes"` on an `input` of type `submit` or `button` (`value` is not a readable
  attribute, because on other inputs it is data);
- text returned by a Rust function or constant and shown by the screen (FR-023).

I-13 checks these by hand before it removes the table, so SC-001 does not pass with them left.

## Exceptions

Two constants in `crates/dioxus-lint/src/main.rs`, each entry with its reason:

```rust
/// Files whose text is not part of the product.
const EXEMPT_FILES: &[(&str, &str)] = &[
    ("crates/horae/src/pages/gallery.rs", "development page with sample data (spec Edge Cases)"),
];

/// Literals that are not language: shown as they are in every language.
const NOT_LANGUAGE: &[(&str, &str)] = &[
    ("horae_pat_", "token prefix, part of the token format"),
    // product names, currency codes and format patterns as the migration meets them
];
```

A literal matches an entry when its text equals it exactly. Each migration PR adds the entries it
needs; review checks the reasons. The brand ("Horae") is not an exception: it is the `-brand`
term.

## Baseline and output

- I-1 adds `["dioxus/literal-text"]` to `baseline.toml` with a `reason` and one line per file with
  its count at that moment. From then on, the existing ratchet applies: a file whose count rises
  fails with each site's line; a file whose count falls fails until its line is lowered (or
  removed at 0). A new file starts at 0.

- I-13 removes the table. The rule then fails on any loose text outside the exceptions (SC-001).

- After the per-rule totals, the lint prints one informational line that never fails (the
  numbers here are an example):

  ```text
  dioxus/literal-text: 1416 (baseline 1416)
  info: prose literals outside rsx! markup: 588 (toasts, messages built in Rust; not checked)
  ```

  The second count uses the proof of concept's `rust-literal` heuristic over the same files
  (FR-023). Each migration PR quotes both numbers, before and after, in its description.

## Fixtures

`crates/dioxus-lint/fixtures/literal-text/bad.rs` holds one marked line per kind (text node,
readable attribute, prose prop, text inside an `if` arm, a short prop such as `label: "API"`, a
`{"…"}` text, an `if` attribute branch and text under `#[cfg(not(test))]`); `good.rs` holds the
same markup through
`tr.*()` calls, a formatted value with no letters, machine props and a `NOT_LANGUAGE` literal.
`fixtures_flag_their_rule_and_nothing_else` covers them with no change.
