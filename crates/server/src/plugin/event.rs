use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Keep manifest validation, dispatch names, and serialized event tags in one
// catalog so a newly added event is always subscribable.
macro_rules! app_events {
    ($(#[serde(rename = $hook:literal)] $variant:ident {
        $($field:ident: $ty:ty,)*
    },)*) => {
        /// Business events dispatched to subscribed plugins (FR-019).
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(tag = "event")]
        pub enum AppEvent {
            $(#[serde(rename = $hook)] $variant { $($field: $ty,)* },)*
        }

        pub const KNOWN_HOOKS: &[&str] = &[$($hook,)*];

        impl AppEvent {
            /// The hook name matching the plugin manifest's `hooks` entries.
            pub fn hook_name(&self) -> &'static str {
                match self { $(Self::$variant { .. } => $hook,)* }
            }
        }
    };
}

app_events! {
    #[serde(rename = "time_entry_created")]
    TimeEntryCreated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        time_entry: TimeEntryPayload,
    },
    #[serde(rename = "time_entry_stopped")]
    TimeEntryStopped {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        time_entry: TimeEntryPayload,
    },
    #[serde(rename = "invoice_created")]
    InvoiceCreated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        invoice: InvoicePayload,
    },
    #[serde(rename = "invoice_sent")]
    InvoiceSent {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        invoice: InvoicePayload,
    },
    #[serde(rename = "user_logged_in")]
    UserLoggedIn {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        user: UserPayload,
    },
    #[serde(rename = "time_entry_updated")]
    TimeEntryUpdated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        time_entry: TimeEntryPayload,
    },
    #[serde(rename = "time_entry_deleted")]
    TimeEntryDeleted {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        time_entry: TimeEntryPayload,
    },
    #[serde(rename = "timesheet_submitted")]
    TimesheetSubmitted {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        submission: SubmissionPayload,
    },
    #[serde(rename = "submission_approved")]
    SubmissionApproved {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        submission: SubmissionPayload,
    },
    #[serde(rename = "submission_rejected")]
    SubmissionRejected {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        submission: SubmissionPayload,
    },
    #[serde(rename = "invoice_paid")]
    InvoicePaid {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        invoice: InvoicePayload,
    },
    #[serde(rename = "invoice_voided")]
    InvoiceVoided {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        invoice: InvoicePayload,
    },
    #[serde(rename = "client_created")]
    ClientCreated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        client: ClientPayload,
    },
    #[serde(rename = "client_updated")]
    ClientUpdated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        client: ClientPayload,
    },
    #[serde(rename = "client_deactivated")]
    ClientDeactivated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        client: ClientPayload,
    },
    #[serde(rename = "client_reactivated")]
    ClientReactivated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        client: ClientPayload,
    },
    #[serde(rename = "project_created")]
    ProjectCreated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        project: ProjectPayload,
    },
    #[serde(rename = "project_updated")]
    ProjectUpdated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        project: ProjectPayload,
    },
    #[serde(rename = "project_deactivated")]
    ProjectDeactivated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        project: ProjectPayload,
    },
    #[serde(rename = "project_reactivated")]
    ProjectReactivated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        project: ProjectPayload,
    },
    #[serde(rename = "task_created")]
    TaskCreated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        task: TaskPayload,
    },
    #[serde(rename = "task_updated")]
    TaskUpdated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        task: TaskPayload,
    },
    #[serde(rename = "task_deactivated")]
    TaskDeactivated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        task: TaskPayload,
    },
    #[serde(rename = "task_reactivated")]
    TaskReactivated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        task: TaskPayload,
    },
    #[serde(rename = "user_created")]
    UserCreated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        user: UserPayload,
    },
    // An Administrator saved a change to the person's permissions in the
    // permission editor; `user` carries the state after the save.
    #[serde(rename = "user_permissions_changed")]
    UserPermissionsChanged {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        user: UserPayload,
    },
    #[serde(rename = "user_deactivated")]
    UserDeactivated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        user: UserPayload,
    },
    #[serde(rename = "user_logged_out")]
    UserLoggedOut {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        user: UserPayload,
    },
    #[serde(rename = "user_assigned_to_project")]
    UserAssignedToProject {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        assignment: AssignmentPayload,
    },
    #[serde(rename = "assignment_removed")]
    AssignmentRemoved {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        assignment: AssignmentPayload,
    },
    #[serde(rename = "org_branding_updated")]
    OrgBrandingUpdated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        org: OrgBrandingPayload,
    },
    #[serde(rename = "project_budget_threshold_reached")]
    ProjectBudgetThresholdReached {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        budget: BudgetThresholdPayload,
    },
    #[serde(rename = "project_over_budget")]
    ProjectOverBudget {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        budget: BudgetThresholdPayload,
    },
    #[serde(rename = "timer_running_too_long")]
    TimerRunningTooLong {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        running_minutes: i32,
        time_entry: TimeEntryPayload,
    },
}

impl AppEvent {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntryPayload {
    pub id: Uuid,
    pub user_id: Uuid,
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub spent_date: NaiveDate,
    pub minutes: i32,
    pub billable: bool,
    pub is_running: bool,
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoicePayload {
    pub id: Uuid,
    pub client_id: Uuid,
    pub invoice_number: String,
    pub status: String,
    pub issue_date: NaiveDate,
    pub due_date: NaiveDate,
    pub currency: String,
    pub total_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPayload {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    /// The stored Administrator identity, which no grant confers.
    pub is_administrator: bool,
    /// The built-in profile's label or the template's name the person's
    /// permissions come from; `None` for individually chosen grants or a
    /// person without stored permissions.
    pub profile: Option<String>,
    /// The same profile by a stable identity that survives renamed templates
    /// and relabelled built-in profiles; `None` whenever `profile` is.
    pub profile_ref: Option<ProfileRef>,
    /// How the user authenticated — set on login events, and on the account
    /// creation a first OIDC sign-in performs; omitted otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
}

/// The profile a person's permissions come from, as plugins see it:
/// `{"built_in": "accounting"}` or `{"template": "<uuid>"}`. Unlike
/// `PermissionSource`, it leaves out the template revision that was applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileRef {
    BuiltIn(horae_core::permissions::catalog::BuiltInProfile),
    Template(Uuid),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionPayload {
    pub id: Uuid,
    pub user_id: Uuid,
    pub week_start: NaiveDate,
    pub status: String,
    pub total_minutes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientPayload {
    pub id: Uuid,
    pub name: String,
    pub currency: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPayload {
    pub id: Uuid,
    pub client_id: Uuid,
    pub name: String,
    pub project_type: String,
    pub budget_kind: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPayload {
    pub id: Uuid,
    pub name: String,
    pub billable_default: bool,
    pub default_rate_cents: Option<i64>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentPayload {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
}

/// Only non-sensitive branding fields are exposed to plugins (no bank details).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgBrandingPayload {
    pub org_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_name: Option<String>,
}

/// Consumption-vs-budget figures for the derived budget events. Only the fields
/// matching the project's budget kind are set (minutes for `hours`, cents for
/// `amount`); the rest are omitted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetThresholdPayload {
    pub project: ProjectPayload,
    pub threshold_pct: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumed_minutes: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_minutes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumed_cents: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_amount_cents: Option<i64>,
}

/// Which lifecycle event an active-flag write should emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTransition {
    Deactivated,
    Reactivated,
}

/// Decide which event an `active` write represents. Returns `None` when the flag
/// did not actually change, so a no-op set emits nothing (FR-012). `was_active`
/// is the value read before the write (`None` if the row was absent).
pub fn active_transition(was_active: Option<bool>, now_active: bool) -> Option<ActiveTransition> {
    match was_active {
        Some(prev) if prev != now_active => Some(if now_active {
            ActiveTransition::Reactivated
        } else {
            ActiveTransition::Deactivated
        }),
        _ => None,
    }
}

pub(crate) fn time_entry_payload(e: &crate::models::TimeEntry) -> TimeEntryPayload {
    TimeEntryPayload {
        id: e.id,
        user_id: e.user_id,
        project_id: e.project_id,
        task_id: e.task_id,
        spent_date: e.spent_date,
        minutes: e.minutes,
        billable: e.billable,
        is_running: e.is_running,
        notes: e.notes.clone(),
        started_at: e.started_at,
    }
}

pub(crate) fn client_payload(client: &crate::models::Client) -> ClientPayload {
    ClientPayload {
        id: client.id,
        name: client.name.clone(),
        currency: client.currency.clone(),
        active: client.active,
    }
}

pub(crate) fn invoice_payload(inv: &crate::models::Invoice) -> InvoicePayload {
    InvoicePayload {
        id: inv.id,
        client_id: inv.client_id,
        invoice_number: inv.number.clone(),
        status: inv.status.to_string(),
        issue_date: inv.issued_on,
        due_date: inv.due_on,
        currency: inv.currency.clone(),
        total_cents: inv.total_cents,
    }
}

pub(crate) fn submission_payload(
    a: &crate::models::Approval,
    total_minutes: i32,
) -> SubmissionPayload {
    SubmissionPayload {
        id: a.id,
        user_id: a.user_id,
        week_start: a.period_start,
        status: a.state.to_string(),
        total_minutes,
    }
}

pub(crate) fn project_payload(p: &crate::models::Project) -> ProjectPayload {
    ProjectPayload {
        id: p.id,
        client_id: p.client_id,
        name: p.name.clone(),
        project_type: p.project_type.to_string(),
        budget_kind: p.budget_kind.to_string(),
        active: p.active,
    }
}

pub(crate) fn task_payload(t: &crate::models::Task) -> TaskPayload {
    TaskPayload {
        id: t.id,
        name: t.name.clone(),
        billable_default: t.billable_default,
        default_rate_cents: t.default_rate_cents,
        active: t.active,
    }
}

/// The plugin payload for a person, with their stored Administrator identity
/// and the profile their permissions come from. A person without stored
/// permissions, or with a state this build cannot read, is reported as a
/// non-administrator without a profile.
pub async fn user_payload(
    connection: &mut sqlx::PgConnection,
    org_id: Uuid,
    id: Uuid,
    email: &str,
    name: &str,
) -> Result<UserPayload, sqlx::Error> {
    use crate::ops::permissions::PermissionSource;
    use crate::ops::permissions::{PermissionStorageError, load_person_permissions, profile_name};
    let state = match load_person_permissions(&mut *connection, org_id, id).await {
        Ok(state) => state,
        Err(PermissionStorageError::Database(error)) => return Err(error),
        Err(error) => {
            tracing::error!(%error, "Invalid stored permission state");
            None
        }
    };
    let (is_administrator, profile, profile_ref) = match state {
        Some(state) => (
            state.is_administrator,
            profile_name(connection, org_id, state.source).await?,
            match state.source {
                PermissionSource::BuiltIn(profile) => Some(ProfileRef::BuiltIn(profile)),
                PermissionSource::Template { id, .. } => Some(ProfileRef::Template(id)),
                PermissionSource::Individual => None,
            },
        ),
        None => (false, None, None),
    };
    Ok(UserPayload {
        id,
        email: email.to_owned(),
        name: name.to_owned(),
        is_administrator,
        profile,
        profile_ref,
        method: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use horae_core::permissions::catalog::BuiltInProfile;

    #[test]
    fn project_membership_payload_excludes_private_configuration() {
        let payload = AssignmentPayload {
            id: Uuid::nil(),
            project_id: Uuid::nil(),
            user_id: Uuid::nil(),
            role: "lead".into(),
        };
        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            serde_json::json!({
                "id": Uuid::nil(),
                "project_id": Uuid::nil(),
                "user_id": Uuid::nil(),
                "role": "lead",
            })
        );
    }

    fn sample_entry() -> TimeEntryPayload {
        TimeEntryPayload {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            project_id: Uuid::nil(),
            task_id: Uuid::nil(),
            spent_date: NaiveDate::from_ymd_opt(2026, 7, 13).unwrap(),
            minutes: 30,
            billable: true,
            is_running: false,
            notes: None,
            started_at: None,
        }
    }

    #[test]
    fn new_events_have_stable_hook_names() {
        let org = Uuid::nil();
        let at = DateTime::from_timestamp(0, 0).unwrap();
        let cases = [
            (
                AppEvent::TimeEntryUpdated {
                    occurred_at: at,
                    org_id: org,
                    time_entry: sample_entry(),
                },
                "time_entry_updated",
            ),
            (
                AppEvent::TimeEntryDeleted {
                    occurred_at: at,
                    org_id: org,
                    time_entry: sample_entry(),
                },
                "time_entry_deleted",
            ),
        ];
        for (event, hook) in cases {
            assert_eq!(event.hook_name(), hook);
            // The serde tag in the JSON envelope must match the hook name.
            assert!(
                event.to_json().contains(&format!("\"event\":\"{hook}\"")),
                "envelope tag mismatch for {hook}"
            );
        }
    }

    #[test]
    fn submission_event_serializes_payload() {
        let event = AppEvent::TimesheetSubmitted {
            occurred_at: DateTime::from_timestamp(0, 0).unwrap(),
            org_id: Uuid::nil(),
            submission: SubmissionPayload {
                id: Uuid::nil(),
                user_id: Uuid::nil(),
                week_start: NaiveDate::from_ymd_opt(2026, 7, 6).unwrap(),
                status: "submitted".into(),
                total_minutes: 2280,
            },
        };
        let json = event.to_json();
        assert_eq!(event.hook_name(), "timesheet_submitted");
        assert!(json.contains("\"event\":\"timesheet_submitted\""));
        assert!(json.contains("\"total_minutes\":2280"));
        assert!(json.contains("\"week_start\":\"2026-07-06\""));
    }

    #[test]
    fn us2_events_have_stable_hook_names() {
        let org = Uuid::nil();
        let at = DateTime::from_timestamp(0, 0).unwrap();
        let client = ClientPayload {
            id: Uuid::nil(),
            name: "Acme".into(),
            currency: "EUR".into(),
            active: false,
        };
        let e = AppEvent::ClientDeactivated {
            occurred_at: at,
            org_id: org,
            client: client.clone(),
        };
        assert_eq!(e.hook_name(), "client_deactivated");
        assert!(e.to_json().contains("\"event\":\"client_deactivated\""));
        assert_eq!(
            AppEvent::ClientReactivated {
                occurred_at: at,
                org_id: org,
                client,
            }
            .hook_name(),
            "client_reactivated"
        );
    }

    #[test]
    fn permissions_change_event_carries_the_profile_and_identity() {
        let event = AppEvent::UserPermissionsChanged {
            occurred_at: DateTime::from_timestamp(0, 0).unwrap(),
            org_id: Uuid::nil(),
            user: UserPayload {
                id: Uuid::nil(),
                email: "sam@acme.test".into(),
                name: "Sam".into(),
                is_administrator: false,
                profile: Some("Project Manager".into()),
                profile_ref: Some(ProfileRef::BuiltIn(BuiltInProfile::ProjectManager)),
                method: None,
            },
        };
        let json = event.to_json();
        assert_eq!(event.hook_name(), "user_permissions_changed");
        assert!(json.contains("\"profile\":\"Project Manager\""));
        assert!(json.contains("\"is_administrator\":false"));
        // The login-only `method` field is omitted for admin user events.
        assert!(!json.contains("\"method\""));
    }

    #[test]
    fn individual_permissions_have_no_profile() {
        let user = UserPayload {
            id: Uuid::nil(),
            email: "sam@acme.test".into(),
            name: "Sam".into(),
            is_administrator: true,
            profile: None,
            profile_ref: None,
            method: None,
        };
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("\"profile\":null"), "{json}");
        assert!(json.contains("\"profile_ref\":null"), "{json}");
    }

    /// The plugin contract: the exact JSON of a built-in and a template profile.
    #[test]
    fn profile_ref_serializes_as_a_tagged_identity() {
        let template = Uuid::from_u128(0x0195_0000_0000_7000_8000_0000_0000_00a1);
        assert_eq!(
            serde_json::to_value(ProfileRef::BuiltIn(BuiltInProfile::Accounting)).unwrap(),
            serde_json::json!({ "built_in": "accounting" })
        );
        assert_eq!(
            serde_json::to_value(ProfileRef::Template(template)).unwrap(),
            serde_json::json!({ "template": "01950000-0000-7000-8000-0000000000a1" })
        );
        let user = UserPayload {
            id: Uuid::nil(),
            email: "sam@acme.test".into(),
            name: "Sam".into(),
            is_administrator: false,
            profile: Some("Night shift".into()),
            profile_ref: Some(ProfileRef::Template(template)),
            method: None,
        };
        assert_eq!(
            serde_json::to_value(user).unwrap(),
            serde_json::json!({
                "id": "00000000-0000-0000-0000-000000000000",
                "email": "sam@acme.test",
                "name": "Sam",
                "is_administrator": false,
                "profile": "Night shift",
                "profile_ref": { "template": "01950000-0000-7000-8000-0000000000a1" },
            })
        );
    }

    /// `user_payload` reads the reference from the stored permission source:
    /// a built-in profile, a template, and none for individual grants or a
    /// person without stored permissions.
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn user_payload_carries_the_profile_ref_of_the_stored_source(pool: sqlx::PgPool) {
        use crate::testing::{clear_state, seed};

        async fn payload(pool: &sqlx::PgPool, org_id: Uuid, id: Uuid) -> UserPayload {
            let mut connection = pool.acquire().await.unwrap();
            user_payload(&mut connection, org_id, id, "sam@acme.test", "Sam")
                .await
                .unwrap()
        }

        let ids = seed(&pool, BuiltInProfile::Accounting).await;
        let user = payload(&pool, ids.org_id, ids.user_id).await;
        assert_eq!(
            (user.profile.as_deref(), user.profile_ref),
            (
                Some("Accounting"),
                Some(ProfileRef::BuiltIn(BuiltInProfile::Accounting))
            )
        );

        let template = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO permission_templates (id, org_id, name, catalog_version, grants)
             SELECT $1, org_id, 'Night shift', catalog_version, grants
               FROM person_permission_states WHERE user_id = $2",
            template,
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE person_permission_states
                SET source = 'template', built_in_profile = NULL, template_id = $2,
                    applied_template_revision = 0
              WHERE user_id = $1",
            ids.user_id,
            template
        )
        .execute(&pool)
        .await
        .unwrap();
        let user = payload(&pool, ids.org_id, ids.user_id).await;
        assert_eq!(
            (user.profile.as_deref(), user.profile_ref),
            (Some("Night shift"), Some(ProfileRef::Template(template)))
        );

        sqlx::query!(
            "UPDATE person_permission_states
                SET source = 'individual', template_id = NULL, applied_template_revision = NULL
              WHERE user_id = $1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let user = payload(&pool, ids.org_id, ids.user_id).await;
        assert_eq!((user.profile, user.profile_ref), (None, None));

        clear_state(&pool, ids.user_id).await;
        let user = payload(&pool, ids.org_id, ids.user_id).await;
        assert_eq!((user.profile, user.profile_ref), (None, None));
    }

    #[test]
    fn budget_and_timer_events_serialize() {
        let project = ProjectPayload {
            id: Uuid::nil(),
            client_id: Uuid::nil(),
            name: "Website".into(),
            project_type: "time_and_materials".into(),
            budget_kind: "hours".into(),
            active: true,
        };
        let over = AppEvent::ProjectOverBudget {
            occurred_at: DateTime::from_timestamp(0, 0).unwrap(),
            org_id: Uuid::nil(),
            budget: BudgetThresholdPayload {
                project,
                threshold_pct: 100,
                consumed_minutes: Some(6000),
                budget_minutes: Some(4800),
                consumed_cents: None,
                budget_amount_cents: None,
            },
        };
        assert_eq!(over.hook_name(), "project_over_budget");
        let j = over.to_json();
        assert!(j.contains("\"event\":\"project_over_budget\""));
        assert!(j.contains("\"threshold_pct\":100"));
        assert!(j.contains("\"consumed_minutes\":6000"));
        // Cents fields are omitted for an hours budget.
        assert!(!j.contains("consumed_cents"));

        let timer = AppEvent::TimerRunningTooLong {
            occurred_at: DateTime::from_timestamp(0, 0).unwrap(),
            org_id: Uuid::nil(),
            running_minutes: 495,
            time_entry: sample_entry(),
        };
        assert_eq!(timer.hook_name(), "timer_running_too_long");
        assert!(timer.to_json().contains("\"running_minutes\":495"));
    }

    #[test]
    fn active_transition_fires_only_on_a_real_flip() {
        use ActiveTransition::{Deactivated, Reactivated};
        assert_eq!(active_transition(Some(true), false), Some(Deactivated));
        assert_eq!(active_transition(Some(false), true), Some(Reactivated));
        // No-op sets (same value) emit nothing (FR-012).
        assert_eq!(active_transition(Some(true), true), None);
        assert_eq!(active_transition(Some(false), false), None);
        // Absent prior row → no event.
        assert_eq!(active_transition(None, true), None);
        assert_eq!(active_transition(None, false), None);
    }
}
