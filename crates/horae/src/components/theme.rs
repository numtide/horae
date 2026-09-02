use dioxus::prelude::*;

/// The two palettes defined in horae.css (`:root` plus the `[data-theme="light"]`
/// override). Persisted client-side under `localStorage['horae-theme']` by
/// the script `THEME_SCRIPT` installs — see [`ThemeInit`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub const ALL: [Theme; 2] = [Theme::Dark, Theme::Light];

    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
        }
    }

    /// Anything unrecognised — including the retired `pine` — reads as dark, so
    /// a stale value in a returning browser's storage resolves cleanly.
    pub fn from_str(s: &str) -> Theme {
        match s {
            "light" => Theme::Light,
            _ => Theme::Dark,
        }
    }
}

/// Reads the saved theme (or defaults to dark) and applies it before first
/// paint, and defines `setHoraeTheme()` for the settings page to call. Lives
/// in `<head>` via `document::Script` so it runs ahead of body paint — no
/// flash of the wrong theme on reload. Mirrors `site/index.html`.
const THEME_SCRIPT: &str = r#"(function () {
  window.setHoraeTheme = function (t) {
    document.documentElement.dataset.theme = t;
    localStorage.setItem('horae-theme', t);
  };
  var saved = localStorage.getItem('horae-theme');
  document.documentElement.dataset.theme = saved === 'light' ? 'light' : 'dark';
})();"#;

#[component]
pub fn ThemeInit() -> Element {
    rsx! {
        document::Script { "{THEME_SCRIPT}" }
    }
}
