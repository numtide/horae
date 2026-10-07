use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
};

use super::*;

#[derive(Clone)]
struct Harness {
    ready: Rc<Cell<bool>>,
    committed: Rc<RefCell<Vec<CellEdit>>>,
}

#[test]
fn typing_in_another_cell_survives_a_save_refresh() {
    use dioxus::dioxus_core::Mutation;
    use dioxus::html::{SerializedFocusData, SerializedFormData, SerializedHtmlEventConverter};

    dioxus::html::set_event_converter(Box::new(SerializedHtmlEventConverter));
    let ready = Rc::new(Cell::new(true));
    let committed = Rc::new(RefCell::new(Vec::<CellEdit>::new()));
    let mut dom = VirtualDom::new_with_props(
        |Harness { ready, committed }: Harness| {
            let actions = WeekActions {
                commit: use_callback(move |edit: CellEdit| committed.borrow_mut().push(edit)),
                remove_row: use_callback(|_: (Uuid, Uuid)| {}),
                removing_row: use_signal(|| false).into(),
                add_row: use_callback(|()| {}),
                drafts: use_signal(HashMap::new),
                saving: use_signal(HashSet::new).into(),
                tracking: use_memo(|| {
                    vec![crate::models::scoped_time::TimesheetTrackingOption {
                        project_id: Uuid::nil(),
                        project_name: "Project".into(),
                        task_id: Uuid::nil(),
                        task_name: "Task".into(),
                        billable: true,
                    }]
                }),
                busy: use_memo(|| false),
                policy: use_memo(|| Some(TimesheetPolicy::Scoped)),
            };
            if !ready.get() {
                return rsx! { div { "Loading timesheet…" } };
            }
            let start = "2026-09-07".parse().unwrap();
            render_week_view(
                &[],
                &[0; 7],
                start,
                start,
                &HashMap::new(),
                &HashMap::new(),
                &[(Uuid::nil(), Uuid::nil())],
                actions,
            )
        },
        Harness {
            ready: ready.clone(),
            committed: committed.clone(),
        },
    );
    let initial = dom.rebuild_to_vec();
    let second_cell = initial
        .edits
        .iter()
        .filter_map(|edit| match edit {
            Mutation::NewEventListener { name, id } if name == "blur" => Some(*id),
            _ => None,
        })
        .nth(1)
        .unwrap();
    // B receives input but not blur/change before A's response refreshes the grid.
    let event = Event::new(
        Rc::new(PlatformEventData::new(Box::new(SerializedFormData::new(
            "2:00".into(),
            vec![],
        )))) as Rc<dyn Any>,
        true,
    );
    dom.runtime().handle_event("input", event, second_cell);
    dom.render_immediate_to_vec();
    assert!(committed.borrow().is_empty());
    ready.set(false);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate_to_vec();
    assert!(!dioxus::ssr::render(&dom).contains("ts-cell-input"));
    ready.set(true);
    dom.mark_dirty(ScopeId::APP);
    let restored = dom.render_immediate_to_vec();
    assert!(
        dioxus::ssr::render(&dom).contains("value=\"2:00\""),
        "unsaved B must survive A's refresh"
    );
    let second_cell = restored
        .edits
        .iter()
        .filter_map(|edit| match edit {
            Mutation::NewEventListener { name, id } if name == "blur" => Some(*id),
            _ => None,
        })
        .nth(1)
        .unwrap();
    dom.runtime().handle_event(
        "blur",
        Event::new(
            Rc::new(PlatformEventData::new(Box::new(SerializedFocusData {}))) as Rc<dyn Any>,
            false,
        ),
        second_cell,
    );
    assert_eq!(committed.borrow().len(), 1);
    assert_eq!(committed.borrow()[0].input, "2:00");
    assert_eq!(
        committed.borrow()[0].day,
        "2026-09-08".parse::<NaiveDate>().unwrap()
    );
}

#[test]
fn discarding_a_row_removes_its_drafts_only_for_the_captured_week() {
    let start: NaiveDate = "2026-09-07".parse().unwrap();
    let row = (Uuid::now_v7(), Uuid::now_v7());
    let other_task = Uuid::now_v7();
    let mut drafts = HashMap::from([
        ((row.0, row.1, start), "invalid".into()),
        ((row.0, row.1, start + Duration::days(6)), "2:00".into()),
        ((row.0, row.1, start + Duration::days(7)), "3:00".into()),
        ((row.0, row.1, start - Duration::days(1)), "4:00".into()),
        ((row.0, other_task, start), "5:00".into()),
    ]);
    discard_row_drafts(&mut drafts, row, start);
    assert_eq!(
        drafts,
        HashMap::from([
            ((row.0, row.1, start + Duration::days(7)), "3:00".into()),
            ((row.0, row.1, start - Duration::days(1)), "4:00".into()),
            ((row.0, other_task, start), "5:00".into()),
        ])
    );
}
