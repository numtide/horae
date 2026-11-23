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
    let tr = crate::i18n::use_tr();
    let me = use_session().me;
    // The router renders Profile in place at `/settings`; give the address
    // bar the subpage's own URL, so a copied link names it.
    use_effect(|| {
        let profile = Route::SettingsProfile {}.to_string();
        document::eval(&format!(
            "if (/^\\/settings\\/?$/.test(location.pathname)) history.replaceState(history.state, '', {profile:?});"
        ));
    });
    let name = me.read().name.clone();
    rsx! {
        div { class: "flex flex-wrap items-start gap-12",
            aside { class: "adm-nav",
                div { class: "adm-head flex items-center gap-3",
                    Avatar { initials: initials(&name) }
                    div { class: "min-w-0 text-sm font-semibold truncate", "{name}" }
                }
                div { class: "adm-group-label", {tr.settings_group_settings()} }
                nav { class: "flex flex-col gap-1 mb-5", aria_label: tr.settings_group_settings(),
                    AdmLink { to: Route::SettingsProfile {}, label: tr.settings_profile_title() }
                    AdmLink { to: Route::SettingsPreferences {}, label: tr.settings_preferences_title() }
                    AdmLink { to: Route::SettingsApiTokens {}, label: tr.api_tokens_title() }
                }
                div { class: "adm-group-label", {tr.settings_group_account()} }
                nav { class: "flex flex-col gap-1 mb-5", aria_label: tr.settings_group_account(),
                    AdmLink { to: Route::SettingsAccess {}, label: tr.settings_access_title() }
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
    let tr = crate::i18n::use_tr();
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
        PageHeader { title: tr.settings_preferences_title() }
        section { class: "card max-w-form", aria_label: tr.settings_preferences_title(),
            FormGroup { label: tr.settings_theme_label(), id: "theme", hint: tr.settings_theme_hint(),
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
