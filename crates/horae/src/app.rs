use dioxus::prelude::*;

use crate::components::theme::ThemeInit;
use crate::route::Route;

#[allow(non_snake_case)]
pub fn App() -> Element {
    rsx! {
        document::Link {
            rel: "stylesheet",
            href: asset!("/assets/css/horae.css"),
        }
        // Generated utility layer (built by crates/horae/build.rs); tokens/components live in horae.css.
        document::Link {
            rel: "stylesheet",
            href: asset!("/assets/css/horae-utils.css"),
        }
        // Keep in step with the login page's icon links (auth/page.rs).
        document::Link { rel: "icon", href: "/favicon.ico", sizes: "32x32" }
        document::Link {
            rel: "icon",
            r#type: "image/svg+xml",
            href: asset!("/assets/favicon.svg"),
        }
        document::Link {
            rel: "apple-touch-icon",
            href: asset!("/assets/apple-touch-icon.png"),
        }
        ThemeInit {}
        document::Script { src: asset!("/assets/js/project-edit-navigation.js") }
        Router::<Route> {}
    }
}
