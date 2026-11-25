use dioxus::prelude::*;

/// A value shown once, such as a new token or app secret, read-only with a
/// copy button. `label` names the field and `copy_label` its button.
#[component]
pub fn RevealedSecret(
    id: &'static str,
    value: String,
    label: String,
    copy_label: String,
) -> Element {
    let tr = crate::i18n::use_tr();
    let mut copied = use_signal(|| false);
    let copy = {
        let value = value.clone();
        move |_| {
            let js = format!(
                "navigator.clipboard.writeText({});",
                serde_json::to_string(&value).unwrap_or_default()
            );
            document::eval(&js);
            copied.set(true);
        }
    };
    rsx! {
        div { class: "input-group",
            input { id, aria_label: label,
                class: "input-group-field text-mono text-xs truncate", readonly: true, value: "{value}",
            }
            button { id: "{id}-copy", r#type: "button", class: "btn btn-secondary btn-sm",
                aria_label: if copied() { tr.action_copied() } else { copy_label },
                onclick: copy,
                if copied() { {tr.action_copied()} } else { {tr.action_copy()} }
            }
        }
    }
}
