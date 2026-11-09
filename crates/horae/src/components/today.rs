//! The signed-in person's "today" in the UI, in the zone their profile names.

use chrono::{Duration, NaiveDate, Utc};
use dioxus::prelude::*;

/// Seconds the person's zone is ahead of UTC, once the session has loaded.
/// Provided by the app layout from `get_me`.
#[derive(Clone, Copy)]
pub struct UtcOffset(pub Memo<Option<i32>>);

/// Today's date for the signed-in person. Reading it in a component re-renders
/// that component when the session's offset arrives.
///
/// shortcut: the offset is the one `get_me` saw, so a tab left open across a
/// daylight-saving change is an hour off until the session is re-read (opening
/// the account menu, or a reload).
pub fn today() -> NaiveDate {
    // Outside a component (route parsing in tests, server code) there is no
    // context to ask, and try_consume_context would panic without a runtime.
    let in_scope = dioxus::core::Runtime::try_current()
        .is_some_and(|runtime| runtime.try_current_scope_id().is_some());
    let offset = in_scope
        .then(try_consume_context::<UtcOffset>)
        .flatten()
        .and_then(|UtcOffset(offset)| offset());
    match offset {
        Some(seconds) => (Utc::now() + Duration::seconds(seconds.into())).date_naive(),
        None => before_session(),
    }
}

/// Before the session loads (the first redirect from `/` happens then) the
/// browser's own zone is the best guess; the server renders in UTC.
fn before_session() -> NaiveDate {
    #[cfg(feature = "web")]
    return chrono::Local::now().date_naive();
    #[cfg(not(feature = "web"))]
    Utc::now().date_naive()
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    #[derive(Clone)]
    struct Probe {
        offset: Option<i32>,
        seen: Rc<Cell<Option<NaiveDate>>>,
    }

    fn app() -> Element {
        let probe = use_context::<Probe>();
        let offset = use_memo(move || probe.offset);
        use_context_provider(|| UtcOffset(offset));
        rsx! { Reader {} }
    }

    #[component]
    fn Reader() -> Element {
        use_context::<Probe>().seen.set(Some(today()));
        rsx! {}
    }

    fn rendered_today(offset: Option<i32>) -> NaiveDate {
        let seen = Rc::new(Cell::new(None));
        let probe = Probe {
            offset,
            seen: seen.clone(),
        };
        let mut dom = VirtualDom::new(app).with_root_context(probe);
        dom.rebuild_in_place();
        seen.get().unwrap()
    }

    fn date_at_offset(hours: i64) -> NaiveDate {
        (Utc::now() + Duration::hours(hours)).date_naive()
    }

    #[test]
    fn follows_the_sessions_offset_once_loaded() {
        // Read either side of the render in case midnight passes meanwhile.
        let before = date_at_offset(14);
        let day = rendered_today(Some(14 * 3600));
        let after = date_at_offset(14);
        assert!([before, after].contains(&day), "{day}");
        // UTC+14 and UTC−12 are never on the same day.
        assert_ne!(
            rendered_today(Some(14 * 3600)),
            rendered_today(Some(-12 * 3600))
        );
    }

    #[test]
    fn falls_back_to_utc_on_the_server_before_the_session_loads() {
        assert_eq!(rendered_today(None), Utc::now().date_naive());
    }

    #[test]
    fn works_outside_any_component() {
        assert_eq!(today(), Utc::now().date_naive());
    }
}
