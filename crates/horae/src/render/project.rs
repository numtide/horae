//! Selected-period project work summary, using the dashboard's authorized totals.

use crate::models::project::{ProjectBreakdown, ProjectWorkEntity};
use horae_core::{duration::format_hours2, money::format_cents, project_breakdown::WorkTotals};
use typst::foundations::{Dict, IntoValue, Value};
use typst_as_lib::TypstEngine;

pub(crate) fn fits_project_pdf(name: &str, client: &str, data: &ProjectBreakdown) -> bool {
    if data.tasks.len() + data.people.len() > 1_000 {
        return false;
    }
    let fields = [name, client, data.cost_currency.as_deref().unwrap_or("")]
        .into_iter()
        .chain(
            data.tasks
                .iter()
                .chain(&data.people)
                .map(|entity| entity.name.as_str()),
        );
    let mut bytes = 0usize;
    for field in fields {
        if field.len() > 32_767 {
            return false;
        }
        bytes += field.len();
        if bytes > 1024 * 1024 {
            return false;
        }
    }
    true
}

fn row(name: &str, totals: WorkTotals, currency: Option<&str>) -> Value {
    let mut row = Dict::new();
    row.insert("name".into(), name.to_owned().into_value());
    row.insert("hours".into(), format_hours2(totals.minutes).into_value());
    row.insert("minutes".into(), totals.minutes.to_string().into_value());
    let cost = match (currency, totals.cost_cents) {
        (Some(currency), Some(cents)) => format_cents(cents, currency),
        (Some(_), None) => "Missing cost rates".into(),
        (None, _) => "Restricted".into(),
    };
    row.insert("cost".into(), cost.into_value());
    Value::Dict(row)
}

fn inputs(name: &str, client: &str, data: &ProjectBreakdown) -> Dict {
    let mut inputs = Dict::new();
    inputs.insert("project".into(), name.to_owned().into_value());
    inputs.insert("client".into(), client.to_owned().into_value());
    inputs.insert(
        "period".into(),
        data.interval
            .map_or_else(
                || "All time".into(),
                |range| format!("{} – {} (inclusive)", range.from, range.to),
            )
            .into_value(),
    );
    let currency = data.cost_currency.as_deref();
    inputs.insert("total".into(), row("Total", data.totals.total, currency));
    inputs.insert(
        "billable".into(),
        format_hours2(data.totals.total.billable_minutes).into_value(),
    );
    inputs.insert(
        "non_billable".into(),
        format_hours2(data.totals.total.minutes - data.totals.total.billable_minutes).into_value(),
    );
    for (key, entities, totals) in [
        ("tasks", &data.tasks, &data.totals.by_task),
        ("people", &data.people, &data.totals.by_person),
    ] {
        let rows: Vec<_> = entities
            .iter()
            .map(|entity: &ProjectWorkEntity| {
                let label = if entity.current && entity.active {
                    entity.name.clone()
                } else {
                    format!("{} (Historical)", entity.name)
                };
                row(
                    &label,
                    totals
                        .get(&entity.id)
                        .copied()
                        .unwrap_or_else(|| WorkTotals::empty(currency.is_some())),
                    currency,
                )
            })
            .collect();
        inputs.insert(key.into(), Value::Array(rows.as_slice().into()));
    }
    inputs
}

fn document(
    name: &str,
    client: &str,
    data: &ProjectBreakdown,
) -> anyhow::Result<typst_layout::PagedDocument> {
    anyhow::ensure!(
        fits_project_pdf(name, client, data),
        "Project summary exceeds PDF limits"
    );
    let engine = TypstEngine::builder()
        .main_file(include_str!("../../templates/project-summary.typ"))
        .search_fonts_with(
            typst_as_lib::typst_kit_options::TypstKitFontOptions::default()
                .include_system_fonts(false)
                .include_embedded_fonts(true),
        )
        .build();
    engine
        .compile_with_input(inputs(name, client, data))
        .output
        .map_err(|error| anyhow::anyhow!("Project summary compilation failed: {error}"))
}

pub(crate) fn render_project_pdf(
    name: &str,
    client: &str,
    data: &ProjectBreakdown,
) -> anyhow::Result<Vec<u8>> {
    let document = document(name, client, data)?;
    typst_pdf::pdf(
        &document,
        &typst_pdf::PdfOptions {
            timestamp: None,
            ..Default::default()
        },
    )
    .map_err(|error| anyhow::anyhow!("Project summary PDF failed: {error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::ProjectWorkEntity;
    use horae_core::project_breakdown::{WorkCell, WorkTotals, summarize};
    use uuid::Uuid;

    fn fixture() -> ProjectBreakdown {
        let task = Uuid::from_u128(1);
        let person = Uuid::from_u128(2);
        let cells = vec![WorkCell {
            task_id: task,
            user_id: person,
            totals: WorkTotals {
                minutes: 90,
                billable_minutes: 60,
                cost_cents: None,
            },
        }];
        let entity = |id| ProjectWorkEntity {
            id,
            name: "Café #panic(\"not code\")".into(),
            active: true,
            current: true,
            manager: false,
        };
        ProjectBreakdown {
            interval: None,
            cost_currency: None,
            tasks: vec![entity(task)],
            people: vec![entity(person)],
            totals: summarize(&cells, false).unwrap(),
            cells,
        }
    }

    #[test]
    fn project_summary_is_a_deterministic_pdf_with_literal_user_text() {
        let data = fixture();
        let first = render_project_pdf("Project #panic(\"text\")", "Client <&>", &data).unwrap();
        assert!(first.starts_with(b"%PDF-"));
        assert_eq!(
            first,
            render_project_pdf("Project #panic(\"text\")", "Client <&>", &data).unwrap()
        );
    }

    #[test]
    fn project_summary_limits_reject_whole_documents_without_truncation() {
        let mut data = fixture();
        assert!(fits_project_pdf("Project", "Client", &data));
        assert!(!fits_project_pdf(&"x".repeat(32_768), "Client", &data));
        data.tasks = vec![data.tasks[0].clone(); 1_000];
        assert!(!fits_project_pdf("Project", "Client", &data));
        data.tasks.truncate(999);
        assert!(fits_project_pdf("Project", "Client", &data));
        for entity in &mut data.tasks {
            entity.name = "x".repeat(2_000);
        }
        assert!(!fits_project_pdf("Project", "Client", &data));
    }

    #[test]
    fn project_summary_preserves_exact_totals_period_and_private_costs() {
        fn visit(frame: &typst::layout::Frame, text: &mut String) {
            for (_, item) in frame.items() {
                match item {
                    typst::layout::FrameItem::Group(group) => visit(&group.frame, text),
                    typst::layout::FrameItem::Text(run) => {
                        text.push_str(&run.text);
                        text.push('\n');
                    }
                    _ => {}
                }
            }
        }
        let mut data = fixture();
        data.interval = Some(crate::models::project::ProjectActivityInterval {
            from: "2026-09-01".parse().unwrap(),
            to: "2026-09-30".parse().unwrap(),
        });
        // Even a stray amount must not bypass the explicit currency/authority marker.
        data.totals.total.cost_cents = Some(987654321);
        let pdf = document("Project", "Client", &data).unwrap();
        let mut text = String::new();
        for page in pdf.pages() {
            visit(&page.frame, &mut text);
        }
        for expected in [
            "2026-09-01",
            "2026-09-30",
            "1.50",
            "90",
            "Restricted",
            "Tasks",
            "Team",
        ] {
            assert!(text.contains(expected), "Missing {expected}: {text}");
        }
        assert!(!text.contains("987654"));
    }
}
