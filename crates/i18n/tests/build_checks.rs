//! Each check of `build.rs` refuses its case with an error naming the id.

#[allow(dead_code)]
#[path = "../build/checks.rs"]
mod checks;

use checks::{check_translation, parse};

fn error(source: &str) -> String {
    parse("horae.ftl", source).expect_err("the catalog is refused")
}

#[test]
fn a_syntax_error_is_refused() {
    let e = error("ok = Fine\nbroken = { $name\n");
    assert!(e.contains("syntax error"), "{e}");
}

#[test]
fn a_duplicate_id_is_refused() {
    let e = error("tasks-title = Tasks\ntasks-title = Tasks again\n");
    assert!(e.contains("`tasks-title` is defined twice"), "{e}");
}

#[test]
fn a_message_without_a_value_is_refused() {
    let e = error("tasks-title =\n    .aria-label = Tasks\n");
    assert!(e.contains("`tasks-title` has no value"), "{e}");
}

#[test]
fn a_reference_to_an_unknown_message_is_refused() {
    let e = error("tasks-title = { tasks-missing } list\n");
    assert!(
        e.contains("`tasks-title` references unknown message `tasks-missing`"),
        "{e}"
    );
}

#[test]
fn a_reference_to_an_unknown_term_is_refused() {
    let e = error("tasks-title = { -missing } tasks\n");
    assert!(
        e.contains("`tasks-title` references unknown term `-missing`"),
        "{e}"
    );
}

#[test]
fn two_ids_with_one_method_name_are_refused() {
    let e = error("tasks-title = Tasks\ntasks_title = Tasks\n");
    assert!(e.contains("both become the method `tasks_title`"), "{e}");
}

#[test]
fn a_keyword_variable_is_refused() {
    let e = error("tasks-kind = Kind { $type }\n");
    assert!(e.contains("`tasks-kind` uses the variable `$type`"), "{e}");
}

#[test]
fn variables_come_through_referenced_messages() {
    let catalog = parse(
        "horae.ftl",
        "a-inner = { $count }\na-outer = { a-inner } of { $total }\n",
    )
    .unwrap();
    let vars: Vec<&str> = catalog["a-outer"].vars.iter().map(String::as_str).collect();
    assert_eq!(vars, ["count", "total"]);
}

#[test]
fn the_text_keeps_placeables_and_the_default_variant() {
    let source = "a-days = { $n ->\n    [one] { $n } day\n   *[other] { $n } days\n}\na-brand = { -brand } {\"{\"}\n-brand = Horae\n";
    let catalog = parse("horae.ftl", source).unwrap();
    assert_eq!(catalog["a-days"].text, "{ $n } days");
    assert_eq!(catalog["a-brand"].text, "{ -brand } {");
}

fn english() -> checks::Catalog {
    parse("en/horae.ftl", "a-one = One\na-name = Hello { $name }\n").unwrap()
}

#[test]
fn a_translation_with_an_extra_message_is_refused() {
    let translated = parse("xx/horae.ftl", "a-extra = X\n").unwrap();
    let e = check_translation("xx/horae.ftl", &english(), &translated).unwrap_err();
    assert!(e.contains("`a-extra` is not in the English catalog"), "{e}");
}

#[test]
fn a_translation_with_other_variables_is_refused() {
    let translated = parse("xx/horae.ftl", "a-name = Hola { $person }\n").unwrap();
    let e = check_translation("xx/horae.ftl", &english(), &translated).unwrap_err();
    assert!(e.contains("`a-name` uses"), "{e}");
}

#[test]
fn a_translation_that_lacks_messages_counts_them() {
    let translated = parse("xx/horae.ftl", "a-one = Uno\n").unwrap();
    assert_eq!(
        check_translation("xx/horae.ftl", &english(), &translated),
        Ok(1)
    );
}
