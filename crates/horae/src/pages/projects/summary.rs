use super::*;
use crate::models::project::ProjectSummary;

pub(super) fn chart_budget(
    summary: &ProjectSummary,
    interval: Option<crate::models::project::ProjectActivityInterval>,
) -> Option<activity::HourBudget> {
    let row = summary.budgets.first()?;
    let minutes = summary.budget_totals.budget.filter(|value| *value >= 0)?;
    if summary.budget_totals.unallocated_scopes != 0
        || summary.budgets.iter().any(|other| {
            other.kind != BudgetKind::Hours
                || other.budget.is_none()
                || other.period_key != row.period_key
                || other.scope != row.scope
        })
    {
        return None;
    }
    if row.period_key == "lifetime" {
        if interval.is_some() {
            return None;
        }
    } else {
        let start = format!("{}-01", row.period_key)
            .parse::<chrono::NaiveDate>()
            .ok()?;
        let month = horae_core::project_activity::ActivityRange::month(start).ok()?;
        if interval
            != Some(crate::models::project::ProjectActivityInterval {
                from: month.from(),
                to: month.to(),
            })
        {
            return None;
        }
    }
    let label = match row.scope.as_str() {
        "project" => "Project hours budget",
        "task" => "Combined task hour allowances",
        "person" => "Combined person hour allowances",
        _ => return None,
    };
    Some(activity::HourBudget {
        minutes,
        label,
        configured: summary.configured_budget,
    })
}

#[component]
pub(super) fn ProjectSummaryPanel(
    project_id: Uuid,
    can_manage: bool,
    mut summary: Resource<Result<ProjectSummary, ServerFnError>>,
    children: Element,
) -> Element {
    rsx! {
        section { class: "mt-4", aria_label: "Project summary",
            div { class: "project-summary-grid grid gap-4",
            if summary.state()() != UseResourceState::Ready {
                p { role: "status", "Loading project summary…" }
            } else {
                match &*summary.read() {
                    Some(Ok(summary)) => render_summary(project_id, summary, can_manage),
                    Some(Err(error)) => rsx! {
                        div { class: "card p-5",
                        p { class: "text-danger", role: "alert", "Could not load project summary: {error}" }
                        button { r#type: "button", class: "btn btn-secondary min-h-control", onclick: move |_| summary.restart(), "Retry summary" }
                        }
                    },
                    None => rsx! {},
                }
            }
                {children}
            }
        }
    }
}

fn render_summary(project_id: Uuid, summary: &ProjectSummary, can_manage: bool) -> Element {
    let budget = summary
        .budgets
        .first()
        .filter(|row| row.kind != BudgetKind::None)
        .map(|row| {
            (
                row,
                budget_display(
                    row.kind,
                    &row.currency,
                    summary.budget_totals.budget,
                    Some(summary.budget_totals.consumed),
                    row.period_key != "lifetime",
                ),
            )
        });
    rsx! {
            section { class: "card p-5 min-w-0 wrap-anywhere", aria_labelledby: "project-hours-title",
                h2 { id: "project-hours-title", class: "text-sm font-sans font-normal text-secondary m-0", "Total hours" }
                p { class: "font-mono text-3xl font-semibold text-strong mt-2 mb-4",
                    aria_label: "Lifetime total: {hours(summary.total_minutes)}", "{hours(summary.total_minutes)}"
                }
                dl { class: "text-sm m-0",
                    div { class: "flex flex-wrap justify-between gap-2 py-1",
                        dt { "Billable" } dd { class: "font-mono text-secondary m-0", "{hours(summary.billable_minutes)}" }
                    }
                    div { class: "flex flex-wrap justify-between gap-2 py-1",
                        dt { "Non-billable" } dd { class: "font-mono text-secondary m-0", "{hours(summary.non_billable_minutes)}" }
                    }
                }
                p { class: "text-xs text-muted m-0 mt-3", "Lifetime · actual tracked time" }
            }
            section { class: "card p-5 min-w-0 wrap-anywhere", aria_labelledby: "project-budget-title",
                h2 { id: "project-budget-title", class: "text-sm font-sans font-normal text-secondary m-0", "Budget remaining" }
                if let Some((row, values)) = budget {
                    if summary.budget_totals.budget.is_some() {
                        p { class: if summary.budget_totals.remaining.is_some_and(|value| value < 0) {
                                "font-mono text-3xl font-semibold text-danger mt-2 mb-4"
                            } else { "font-mono text-3xl font-semibold text-strong mt-2 mb-4" },
                            "{values.remaining}"
                        }
                    } else {
                        p { class: "text-sm text-muted mt-2 mb-4", "Budget not fully allocated." }
                    }
                    p { class: "text-sm text-secondary m-0", "Budget: {values.budget} · Spent: {values.spent}" }
                    if let Some(percent) = values.pct {
                        progress { class: "proj-bar w-full mt-3", max: "100", value: "{percent}", aria_label: "Budget used", "{percent}%" }
                    }
                    p { class: "text-xs text-muted m-0 mt-3",
                        if row.period_key == "lifetime" { "Budget period: lifetime" }
                        else { "Budget period: {row.period_key}" }
                    }
                    p { class: "text-xs text-muted m-0 mt-1",
                        if summary.configured_budget { "Uses configured billing rounding and included work." }
                        else if row.kind == BudgetKind::Hours { "Based on actual tracked time, including non-billable work." }
                        else { "Based on billable work at its billing rates." }
                    }
                    if summary.budget_totals.unallocated_scopes > 0 {
                        p { class: "text-xs text-muted m-0 mt-2", "Unallocated scopes: {summary.budget_totals.unallocated_scopes}" }
                    }
                    if summary.budget_totals.overrun_scopes > 0 {
                        p { class: "text-xs text-danger m-0 mt-2", "Over-budget scopes: {summary.budget_totals.overrun_scopes}" }
                    }
                    if row.scope != "project" {
                        details { class: "mt-3 text-sm",
                            summary { class: "cursor-pointer text-primary", "Budget by {row.scope}" }
                            ul { class: "m-0 mt-2 pl-4",
                                for scope in &summary.budgets {
                                    {
                                        let values = budget_display(scope.kind, &scope.currency, scope.budget, Some(scope.consumed), false);
                                        rsx! { li { class: "py-1",
                                            span { class: "font-semibold", "{scope.label.as_deref().unwrap_or(\"No allocated scope\")}: " }
                                            if scope.budget.is_some() { "Budget {values.budget} · Remaining {values.remaining} · " }
                                            else { "No budget set · " }
                                            "Spent {values.spent}"
                                        } }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    p { class: "text-sm text-muted mt-2 mb-3", "No budget is set on this project." }
                    if can_manage {
                        Link { to: Route::EditProject { id: project_id }, class: "text-sm font-semibold", "Set up a budget" }
                    }
                }
            }
            section { class: "card p-5 min-w-0 wrap-anywhere", aria_labelledby: "project-cost-title",
                h2 { id: "project-cost-title", class: "text-sm font-sans font-normal text-secondary m-0", "Internal costs" }
                if let Some(cost) = &summary.internal_costs {
                    if let Some(total) = cost.total_cents {
                        p { class: "font-mono text-3xl font-semibold text-strong mt-2 mb-3", "{format_cents(total, &cost.currency)}" }
                        p { class: "text-xs text-muted m-0", "Lifetime · {cost.currency} · Actual tracked time at current cost rates" }
                    } else {
                        p { class: "font-mono text-3xl font-semibold text-faint mt-2 mb-3", "N/A" }
                        p { class: "text-xs text-muted m-0", "Cost rates are missing for {hours(cost.missing_rate_minutes)} of tracked work. No partial total is shown." }
                    }
                } else {
                    p { class: "font-mono text-3xl font-semibold text-faint mt-2 mb-3", "N/A" }
                    p { class: "text-xs text-muted m-0", "Not available with your permissions." }
                }
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project::ProjectCostSummary;

    fn summary() -> ProjectSummary {
        let budgets = vec![ProjectBudgetProgress {
            project_id: Uuid::nil(),
            task_id: None,
            user_id: None,
            scope: "project".into(),
            label: None,
            kind: BudgetKind::Hours,
            currency: "EUR".into(),
            period_key: "2026-09".into(),
            budget: Some(60),
            consumed: 90,
        }];
        ProjectSummary {
            total_minutes: 120,
            billable_minutes: 90,
            non_billable_minutes: 30,
            configured_budget: true,
            budget_totals: horae_core::budget::summarize_scopes(
                budgets.iter().map(|row| (row.budget, row.consumed)),
            )
            .unwrap(),
            budgets,
            internal_costs: Some(ProjectCostSummary {
                currency: "USD".into(),
                total_cents: Some(12345),
                missing_rate_minutes: 0,
            }),
        }
    }

    #[test]
    fn chart_budget_requires_matching_units_period_and_complete_scopes() {
        let mut value = summary();
        let month = crate::models::project::ProjectActivityInterval {
            from: "2026-09-01".parse().unwrap(),
            to: "2026-09-30".parse().unwrap(),
        };
        assert_eq!(chart_budget(&value, Some(month)).unwrap().minutes, 60);
        assert!(chart_budget(&value, None).is_none());
        assert!(
            chart_budget(
                &value,
                Some(crate::models::project::ProjectActivityInterval {
                    to: "2026-09-29".parse().unwrap(),
                    ..month
                })
            )
            .is_none()
        );
        value.budgets[0].period_key = "lifetime".into();
        assert_eq!(chart_budget(&value, None).unwrap().minutes, 60);
        assert!(chart_budget(&value, Some(month)).is_none());
        value.budgets[0].kind = BudgetKind::Amount;
        assert!(chart_budget(&value, None).is_none());
        value.budgets[0].kind = BudgetKind::None;
        assert!(chart_budget(&value, None).is_none());
        value.budgets[0].kind = BudgetKind::Hours;
        value.budget_totals.budget = None;
        assert!(chart_budget(&value, None).is_none());
        value.budget_totals.budget = Some(0);
        assert_eq!(chart_budget(&value, None).unwrap().minutes, 0);
        value.budget_totals.budget = Some(-1);
        assert!(chart_budget(&value, None).is_none());
    }

    #[test]
    fn chart_budget_labels_combined_scopes_and_rejects_mixed_references() {
        let mut value = summary();
        value.budgets[0].scope = "task".into();
        value.budgets[0].period_key = "lifetime".into();
        let budget = chart_budget(&value, None).unwrap();
        assert_eq!(budget.label, "Combined task hour allowances");
        assert!(budget.configured);
        value.budgets[0].scope = "person".into();
        assert_eq!(
            chart_budget(&value, None).unwrap().label,
            "Combined person hour allowances"
        );
        value.budgets.push(value.budgets[0].clone());
        value.budgets[1].period_key = "2026-09".into();
        assert!(chart_budget(&value, None).is_none());
        value.budgets.clear();
        assert!(chart_budget(&value, None).is_none());
    }

    #[test]
    fn tiles_distinguish_lifetime_hours_budget_period_and_workspace_cost_currency() {
        let html = dioxus::ssr::render_element(render_summary(Uuid::nil(), &summary(), false));
        for expected in [
            "Total hours",
            "Lifetime total: 2h",
            "Billable",
            "Non-billable",
            "Budget period: 2026-09",
            "-0.5h",
            "USD 123.45",
            "Actual tracked time at current cost rates",
        ] {
            assert!(html.contains(expected), "missing {expected}: {html}");
        }
        assert!(!html.contains("Set up a budget"));
    }

    #[test]
    fn tiles_do_not_present_missing_or_private_costs_as_zero() {
        let mut value = summary();
        value.internal_costs.as_mut().unwrap().total_cents = None;
        value.internal_costs.as_mut().unwrap().missing_rate_minutes = 30;
        let html = dioxus::ssr::render_element(render_summary(Uuid::nil(), &value, false));
        assert!(
            html.contains("N/A") && html.contains("0.5h of tracked work"),
            "{html}"
        );
        assert!(!html.contains("USD 0.00"));
        value.internal_costs = None;
        let html = dioxus::ssr::render_element(render_summary(Uuid::nil(), &value, false));
        assert!(
            html.contains("Not available with your permissions"),
            "{html}"
        );
        assert!(!html.contains("USD"));
    }
}
