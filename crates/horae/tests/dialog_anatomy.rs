//! Every dialog is a `Modal` (or `ConfirmDialog`), only `Modal` draws a dialog's
//! footer, and nothing asks through the browser's own `confirm`, `alert` or
//! `prompt` (`DESIGN.md`, *Dialogs*). This fails when a file writes the dialog
//! markup or the footer by hand, or calls a native dialog.

use std::fs;
use std::path::{Path, PathBuf};

/// The one file allowed to write the dialog markup.
const MODAL: &str = "src/components/modal.rs";

fn sources(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, found);
        } else if path
            .extension()
            .is_some_and(|ext| ext == "rs" || ext == "js")
        {
            found.push(path);
        }
    }
}

/// A call to the browser's own dialogs, in Rust's `eval` strings or in a script.
fn native_dialog(code: &str) -> bool {
    ["window.confirm", "window.alert", "window.prompt"]
        .iter()
        .any(|call| code.contains(call))
        || ["confirm(", "alert(", "prompt("].iter().any(|call| {
            code.match_indices(call).any(|(at, _)| {
                // A bare call, not a method or a Rust name such as `confirm()`.
                let before = code[..at].chars().next_back();
                let after = code[at + call.len()..].chars().next();
                !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
                    && matches!(after, Some('\'' | '"' | '`'))
            })
        })
}

#[test]
fn only_modal_writes_dialogs_and_nothing_uses_native_ones() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    sources(&root.join("assets/js"), &mut files);
    let mut offences = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        // Test modules render nothing; their fixtures may hold script text on purpose.
        if relative.contains("_tests") {
            continue;
        }
        let text = fs::read_to_string(&file).unwrap();
        for (number, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            let hand_made = relative.ends_with(".rs")
                && relative != MODAL
                && (["modal-actions", "modal-body", "modal-title", "modal-overlay"]
                    .iter()
                    .any(|class| {
                        code.contains(&format!("\"{class}")) || code.contains(&format!(" {class}\""))
                    })
                    || code.starts_with("dialog {")
                    || code.contains(" dialog {")
                    // Popovers that pick a value (SelectField, DateField) are dialogs too.
                    || (code.contains("role: \"dialog\"") && !code.contains("popover")));
            let native = native_dialog(code);
            if hand_made || native {
                offences.push(format!("{relative}:{}: {}", number + 1, code));
            }
        }
    }
    assert!(
        offences.is_empty(),
        "Build dialogs with Modal or ConfirmDialog, give Modal the footer as props \
         (primary, secondary, cancel) or ModalActions, and confirm with ConfirmDialog \
         instead of the browser's own dialogs:\n{}",
        offences.join("\n")
    );
}

#[test]
fn native_dialogs_are_recognised() {
    assert!(native_dialog(
        "document::eval(\"return window.confirm('Discard?');\")"
    ));
    assert!(native_dialog("return confirm('Discard unsaved changes?');"));
    assert!(native_dialog("alert(\"Saved\")"));
    assert!(!native_dialog("open: confirm(), busy: busy(),"));
    assert!(!native_dialog("let confirmed = confirm_dialog(\"x\");"));
}
