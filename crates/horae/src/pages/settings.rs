//! My settings: a side menu of subpages, the way the admin shell has one.

use dioxus::prelude::*;

use crate::components::admin_shell::AdmLink;
use crate::components::avatar::Avatar;
use crate::components::form::{FormGroup, Select};
use crate::components::page_header::PageHeader;
use crate::components::session::use_session;
use crate::components::sidebar::initials;
use crate::components::theme::Theme;
use crate::route::Route;

mod access;
pub(crate) mod api_tokens;
mod profile;

pub use access::SettingsAccess;
pub use api_tokens::SettingsApiTokens;
pub use profile::SettingsProfile;

/// The My settings layout: who is signed in, the SETTINGS and ACCOUNT groups,
/// and the open subpage.
#[component]
pub fn SettingsShell() -> Element {
    let me = use_session().me;
    let name = match &*me.read() {
        Some(Ok(me)) => me.name.clone(),
        _ => String::new(),
    };
    rsx! {
        div { class: "flex flex-wrap items-start gap-12",
            aside { class: "adm-nav",
                div { class: "adm-head flex items-center gap-3",
                    Avatar { initials: initials(&name) }
                    div { class: "min-w-0 text-sm font-semibold truncate", "{name}" }
                }
                div { class: "adm-group-label", "Settings" }
                nav { class: "flex flex-col gap-1 mb-5", aria_label: "Settings",
                    AdmLink { to: Route::SettingsProfile {}, label: "Profile" }
                    AdmLink { to: Route::SettingsPreferences {}, label: "Preferences" }
                    AdmLink { to: Route::SettingsApiTokens {}, label: "API tokens" }
                }
                div { class: "adm-group-label", "Account" }
                nav { class: "flex flex-col gap-1 mb-5", aria_label: "Account",
                    AdmLink { to: Route::SettingsAccess {}, label: "Your access" }
                }
            }
            div { class: "adm-main flex-1 min-w-0",
                Outlet::<Route> {}
            }
        }
    }
}

/// Settings that apply at once and only for this person: for now, the theme.
#[component]
pub fn SettingsPreferences() -> Element {
    let mut theme = use_signal(|| Theme::Dark);

    // The active theme lives in `<html data-theme>` / localStorage (set by
    // ThemeInit before first paint), not in Rust state, so read it back once
    // the client has mounted to show the dropdown's real current value.
    use_effect(move || {
        spawn(async move {
            if let Ok(current) =
                document::eval("return document.documentElement.dataset.theme || 'dark';")
                    .join::<String>()
                    .await
            {
                theme.set(Theme::from_str(&current));
            }
        });
    });

    let theme_options: Vec<(String, String)> = Theme::ALL
        .iter()
        .map(|t| (t.as_str().to_string(), t.label().to_string()))
        .collect();

    rsx! {
        PageHeader { title: "Preferences" }
        section { class: "card", aria_label: "Preferences",
            FormGroup { label: "Theme", id: "theme", hint: "Choose how Horae looks on this device.",
                Select {
                    id: "theme",
                    options: theme_options,
                    selected: theme().as_str().to_string(),
                    onchange: move |e: FormEvent| {
                        let picked = Theme::from_str(&e.value());
                        theme.set(picked);
                        document::eval(&format!("setHoraeTheme('{}')", picked.as_str()));
                    },
                }
            }
        }
    }
}
