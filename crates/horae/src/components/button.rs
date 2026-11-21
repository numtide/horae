use dioxus::core::AttributeValue;
use dioxus::prelude::*;

/// The button looks of the design system (`DESIGN.md`, *Buttons*).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// The main action of a screen or form; at most one per screen or form.
    #[default]
    Primary,
    Secondary,
    /// Billing actions, such as sending an invoice.
    Accent,
    /// Opens a destructive confirmation; never the confirmation itself.
    Danger,
    /// The confirm button of an irreversible action.
    DangerSolid,
    Ghost,
}

impl ButtonVariant {
    pub fn class(self) -> &'static str {
        match self {
            Self::Primary => "btn-primary",
            Self::Secondary => "btn-secondary",
            Self::Accent => "btn-accent",
            Self::Danger => "btn-danger",
            Self::DangerSolid => "btn-danger-solid",
            Self::Ghost => "btn-ghost",
        }
    }
}

/// A styled action button. The text goes in `label`, which is required so the
/// button always has a name; `children` is only for an icon, drawn before the
/// label. Children compare by pointer, so a button with them re-renders whenever
/// its parent does, and one with only a `label` and a `use_callback` handler
/// does not.
///
/// Any native button attribute passes through (`id`, `r#type`, `disabled`,
/// `aria_label`, `"data-testid"`, a `class` of extra utilities…). The type is
/// `button` unless one is given, so it never submits a form by accident.
#[component]
pub fn Button(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] small: bool,
    label: String,
    onclick: Option<EventHandler<MouseEvent>>,
    #[props(extends = GlobalAttributes, extends = button)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let size = if small { " btn-sm" } else { "" };
    let attributes = with_button_defaults(&format!("btn {}{size}", variant.class()), attributes);
    rsx! {
        button {
            onclick: move |event| {
                if let Some(handler) = onclick {
                    handler.call(event);
                }
            },
            ..attributes,
            {children}
            "{label}"
        }
    }
}

/// A square, icon-only button. `label` is its accessible name (aria-label) and
/// replaces any `aria_label` passed; the icon goes in `children`, so it
/// re-renders whenever its parent does. Takes the same native attributes as
/// [`Button`].
#[component]
pub fn IconButton(
    #[props(default = ButtonVariant::Ghost)] variant: ButtonVariant,
    label: String,
    onclick: Option<EventHandler<MouseEvent>>,
    #[props(extends = GlobalAttributes, extends = button)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let mut attributes =
        with_button_defaults(&format!("btn {} btn-icon", variant.class()), attributes);
    attributes.retain(|attribute| attribute.name != "aria-label");
    attributes.push(Attribute::new("aria-label", label, None, false));
    rsx! {
        button {
            onclick: move |event| {
                if let Some(handler) = onclick {
                    handler.call(event);
                }
            },
            ..attributes,
            {children}
        }
    }
}

/// A main action joined to a caret that offers its variants (submit a week with
/// or without notifying). The caret only fires `ontrigger`; opening a menu is the
/// caller's job. Native attributes go to the main button; `disabled` disables
/// the caret too, since it describes the whole control.
#[component]
pub fn SplitButton(
    #[props(default)] variant: ButtonVariant,
    label: String,
    #[props(default = "▾".to_string())] trigger: String,
    onclick: Option<EventHandler<MouseEvent>>,
    ontrigger: Option<EventHandler<MouseEvent>>,
    #[props(extends = GlobalAttributes, extends = button)] attributes: Vec<Attribute>,
) -> Element {
    let class = format!("btn {}", variant.class());
    let disabled = attributes.iter().any(|attribute| {
        attribute.name == "disabled" && matches!(attribute.value, AttributeValue::Bool(true))
    });
    let attributes = with_button_defaults(&class, attributes);
    rsx! {
        div { class: "btn-split",
            button {
                onclick: move |event| {
                    if let Some(handler) = onclick {
                        handler.call(event);
                    }
                },
                ..attributes,
                "{label}"
            }
            button {
                r#type: "button",
                class: "{class}",
                "aria-label": "More actions",
                disabled,
                onclick: move |event| {
                    if let Some(handler) = ontrigger {
                        handler.call(event);
                    }
                },
                "{trigger}"
            }
        }
    }
}

/// Puts the button's classes before any `class` the caller passed and sets
/// `type="button"` unless the caller chose a type. SSR writes a repeated
/// attribute twice and the browser keeps the first, so the two are merged here
/// rather than written next to the spread.
fn with_button_defaults(class: &str, mut attributes: Vec<Attribute>) -> Vec<Attribute> {
    let mut class = class.to_string();
    attributes.retain(|attribute| {
        if attribute.name != "class" {
            return true;
        }
        if let AttributeValue::Text(extra) = &attribute.value {
            class.push(' ');
            class.push_str(extra);
        }
        false
    });
    if !attributes.iter().any(|attribute| attribute.name == "type") {
        attributes.push(Attribute::new("type", "button", None, false));
    }
    attributes.push(Attribute::new("class", class, None, false));
    attributes
}
