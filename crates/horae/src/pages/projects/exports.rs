use super::*;
use crate::models::project::ProjectActivityInterval;

#[derive(Clone, Copy, PartialEq)]
enum Format {
    Csv,
    Excel,
    Pdf,
}

impl Format {
    fn label(self) -> &'static str {
        match self {
            Self::Csv => "CSV",
            Self::Excel => "Excel",
            Self::Pdf => "PDF summary",
        }
    }

    fn url(self, project: Uuid, interval: Option<ProjectActivityInterval>) -> String {
        let dates = interval.map_or_else(String::new, |range| {
            format!("&from={}&to={}", range.from, range.to)
        });
        match self {
            Self::Csv => format!("/api/reports/export/csv?project_id={project}{dates}"),
            Self::Excel => format!("/api/reports/export/xlsx?project_id={project}{dates}"),
            Self::Pdf => format!(
                "/api/projects/{project}/export/pdf?{}",
                dates.trim_start_matches('&')
            ),
        }
    }
}

#[component]
pub(super) fn ProjectExport(
    project_id: Uuid,
    interval: Option<ProjectActivityInterval>,
    disabled: bool,
) -> Element {
    let mut format = use_signal(|| None::<Format>);
    rsx! {
        Menu { id: "project-export", label: "Export", trigger_class: "py-3 min-h-control", align_right: true, disabled,
            for choice in [Format::Csv, Format::Excel, Format::Pdf] {
                MenuItem { onclick: move |_| format.set(Some(choice)), "{choice.label()}" }
            }
        }
        Modal { id: "project-export-dialog", labelledby: "project-export-title", open: format().is_some(),
            focus_fallback: "project-export-trigger", on_dismiss: move |_| format.set(None),
            if let Some(choice) = format() {
                h2 { id: "project-export-title", class: "modal-title", "Export project — {choice.label()}" }
                div { class: "p-4",
                p {
                    if let Some(range) = interval { "Reporting period: {range.from} – {range.to}, inclusive." }
                    else { "Reporting period: All time." }
                }
                p { class: "text-sm text-secondary",
                    match choice {
                        Format::Csv => "Detailed time entries for this project. CSV streams the complete selected period; no partial summary is substituted.",
                        Format::Excel => "Detailed time entries for this project. Excel supports up to 10,000 entries, 8 MiB of source text and 32,767 bytes per field. Larger exports are rejected; choose a shorter period or CSV.",
                        Format::Pdf => "Selected-period task and team summary, including exact minutes and permitted internal costs. PDF supports up to 1,000 task/team rows, 1 MiB of labels and 32,767 bytes per field. It does not include invoices or lifetime budgets. Larger summaries are rejected, never truncated.",
                    }
                }
                if choice != Format::Csv {
                    p { class: "text-sm text-muted", "Generated files are limited to 32 MiB. If the service is busy or times out, return here to retry or choose a shorter period. Your project view stays open." }
                }
                div { class: "modal-actions flex-wrap",
                    button { r#type: "button", class: "btn btn-secondary min-h-control px-3", onclick: move |_| format.set(None), "Cancel" }
                    if !disabled {
                        a { class: "btn btn-primary min-h-control px-3", href: choice.url(project_id, interval), target: "_blank", rel: "noopener", "Download {choice.label()}" }
                    } else {
                        button { class: "btn btn-primary min-h-control px-3", disabled: true, "Download {choice.label()}" }
                    }
                }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_urls_preserve_project_and_inclusive_period_without_invented_dates() {
        let project = Uuid::from_u128(1);
        for format in [Format::Csv, Format::Excel, Format::Pdf] {
            let all = format.url(project, None);
            assert!(all.contains(&project.to_string()));
            assert!(!all.contains("from="));
            let custom = format.url(
                project,
                Some(ProjectActivityInterval {
                    from: "2026-09-01".parse().unwrap(),
                    to: "2026-09-30".parse().unwrap(),
                }),
            );
            assert!(custom.contains("from=2026-09-01&to=2026-09-30"));
        }
    }
}
