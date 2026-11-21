use dioxus::prelude::*;

use super::button::{Button, ButtonVariant};

/// One footer button. Build it with [`ModalAction::click`], [`ModalAction::submit`]
/// or [`ModalAction::link`]; it is Primary unless [`ModalAction::variant`] says
/// otherwise.
#[derive(Clone, PartialEq)]
pub struct ModalAction {
    label: String,
    variant: ButtonVariant,
    id: Option<String>,
    disabled: bool,
    run: ActionRun,
}

#[derive(Clone, PartialEq)]
enum ActionRun {
    Click(Callback<()>),
    /// Submits the form with this id, which may sit anywhere in the dialog.
    Submit(&'static str),
    /// Follows a link, such as a download, then runs the callback.
    Link(String, Callback<()>),
}

impl ModalAction {
    pub fn click(label: impl Into<String>, onclick: Callback<()>) -> Self {
        Self::new(label.into(), ActionRun::Click(onclick))
    }

    pub fn submit(label: impl Into<String>, form: &'static str) -> Self {
        Self::new(label.into(), ActionRun::Submit(form))
    }

    pub fn link(label: impl Into<String>, href: impl Into<String>, onclick: Callback<()>) -> Self {
        Self::new(label.into(), ActionRun::Link(href.into(), onclick))
    }

    fn new(label: String, run: ActionRun) -> Self {
        Self {
            label,
            variant: ButtonVariant::Primary,
            id: None,
            disabled: false,
            run,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// The footer's Cancel: it always dismisses the dialog. Its id defaults to
/// `{modal id}-cancel`.
#[derive(Clone, PartialEq)]
pub struct ModalCancel {
    label: String,
    id: Option<String>,
    disabled: bool,
    autofocus: bool,
}

impl ModalCancel {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            id: None,
            disabled: false,
            autofocus: false,
        }
    }

    /// Opening the dialog focuses Cancel instead of its first control, so a
    /// stray Enter cannot confirm something irreversible.
    pub fn autofocus(mut self) -> Self {
        self.autofocus = true;
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Default for ModalCancel {
    fn default() -> Self {
        Self::new("Cancel")
    }
}

/// What a footer shows, in its fixed order: the primary action first, then an
/// optional second action, then Cancel.
#[derive(Clone, Default, PartialEq)]
struct Footer {
    primary: Option<ModalAction>,
    secondary: Option<ModalAction>,
    cancel: Option<ModalCancel>,
}

/// Actions declared by a component inside the dialog replace the `Modal`'s own.
#[derive(Clone, Copy)]
struct FooterSlot(Signal<Option<Footer>>);

/// Keep mounted while toggling `open`: closing the native dialog restores focus
/// to its opener, and the browser keeps the rest of the page inert while open.
///
/// The footer is drawn here and nowhere else (`DESIGN.md`, *Dialogs*): the
/// `primary` action first, on the left, then `secondary`, then `cancel`, all
/// stretched across the dialog. `cancel` dismisses the dialog and is disabled
/// while `busy`; pass `cancel: None` for a dialog without one. A component
/// inside the dialog that owns the state its buttons need declares them with
/// [`ModalActions`] instead.
#[component]
pub fn Modal(
    id: &'static str,
    labelledby: &'static str,
    open: bool,
    #[props(default = false)] busy: bool,
    #[props(default = false)] large: bool,
    /// Focus this element only when the opener can no longer receive focus.
    #[props(default)]
    focus_fallback: Option<&'static str>,
    /// The id of the text that says what the dialog is about, read on opening.
    #[props(default)]
    describedby: Option<String>,
    #[props(default)] primary: Option<ModalAction>,
    #[props(default)] secondary: Option<ModalAction>,
    #[props(default = Some(ModalCancel::default()))] cancel: Option<ModalCancel>,
    on_dismiss: EventHandler<()>,
    children: Element,
) -> Element {
    let mut backdrop_press = use_signal(|| false);
    let slot = use_context_provider(|| FooterSlot(Signal::new(None)));
    use_effect(use_reactive!(|(open, id, focus_fallback)| {
        let id = serde_json::json!(id);
        let fallback = serde_json::json!(focus_fallback);
        document::eval(&format!(
            "const dialog = document.getElementById({id});
             if (dialog) {{
                 if ({open} && !dialog.open) dialog.showModal();
                 else if (!{open} && dialog.open) {{
                     dialog.close();
                     if ({fallback} && document.activeElement === document.body) {{
                         document.getElementById({fallback})?.focus();
                     }}
                 }}
             }}"
        ));
    }));
    let dismiss = use_callback(move |_: MouseEvent| on_dismiss.call(()));
    let footer = slot.0.read().clone().unwrap_or(Footer {
        primary,
        secondary,
        cancel,
    });

    rsx! {
        dialog {
            id,
            class: "modal-overlay",
            aria_labelledby: labelledby,
            aria_describedby: describedby,
            aria_busy: busy,
            oncancel: move |e| {
                e.prevent_default();
                if !busy { on_dismiss.call(()); }
            },
            onkeydown: move |e| {
                // Escape belongs to this dialog, not the mobile navigation.
                if e.key() == Key::Escape { e.stop_propagation(); }
            },
            onpointerdown: move |_| backdrop_press.set(true),
            onclick: move |_| {
                // Selecting text and releasing outside must not discard a form.
                if backdrop_press() && !busy { on_dismiss.call(()); }
            },
            div {
                class: if large { "modal modal-lg" } else { "modal" },
                onpointerdown: move |e| {
                    backdrop_press.set(false);
                    e.stop_propagation();
                },
                onclick: move |e| e.stop_propagation(),
                {children}
                // Closed, it renders no buttons a page or a test could mistake for content.
                if open && footer != Footer::default() {
                    div { class: "modal-actions",
                        if let Some(action) = footer.primary { FooterButton { action } }
                        if let Some(action) = footer.secondary { FooterButton { action } }
                        if let Some(cancel) = footer.cancel {
                            Button { id: cancel.id.unwrap_or_else(|| format!("{id}-cancel")),
                                variant: ButtonVariant::Secondary, label: cancel.label,
                                disabled: busy || cancel.disabled, "autofocus": cancel.autofocus.then_some(true), onclick: dismiss,
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn FooterButton(action: ModalAction) -> Element {
    let ModalAction {
        label,
        variant,
        id,
        disabled,
        run,
    } = action;
    let callback = match &run {
        ActionRun::Click(callback) | ActionRun::Link(_, callback) => Some(*callback),
        ActionRun::Submit(_) => None,
    };
    let onclick = use_callback(move |_: MouseEvent| {
        if let Some(callback) = callback {
            callback.call(());
        }
    });
    match run {
        ActionRun::Click(_) => rsx! {
            Button { id, variant, label, disabled, onclick }
        },
        ActionRun::Submit(form) => rsx! {
            Button { id, variant, label, disabled, r#type: "submit", form }
        },
        ActionRun::Link(href, _) => rsx! {
            a { id, class: "btn {variant.class()}", href, onclick, "{label}" }
        },
    }
}

/// Declares the dialog's footer from a component inside it, for a form whose
/// buttons depend on state only that component holds. The enclosing `Modal`
/// draws them, in its fixed order, while this is mounted. Mount at most one at
/// a time, and pass handlers made with `use_callback`.
#[component]
pub fn ModalActions(
    #[props(default)] primary: Option<ModalAction>,
    #[props(default)] secondary: Option<ModalAction>,
    #[props(default = Some(ModalCancel::default()))] cancel: Option<ModalCancel>,
) -> Element {
    let FooterSlot(mut slot) = use_context();
    let footer = Footer {
        primary,
        secondary,
        cancel,
    };
    use_effect(use_reactive!(|footer| slot.set(Some(footer))));
    // The Modal may be unmounting too, taking the slot with it.
    use_drop(move || {
        if let Ok(mut current) = slot.try_write() {
            *current = None;
        }
    });
    rsx! {}
}

/// The dialog's title bar. `id` is the `labelledby` its `Modal` was given.
#[component]
pub fn ModalHeader(id: &'static str, title: String) -> Element {
    rsx! {
        h2 { id, class: "modal-title", "{title}" }
    }
}

/// The padded content of a dialog, between its header and its footer.
#[component]
pub fn ModalBody(children: Element) -> Element {
    rsx! {
        div { class: "modal-body", {children} }
    }
}
