//! Creator-owned form state is distinct from operational project records.

use chrono::{DateTime, Utc};
use horae_core::project::{BudgetMode, MonthlyFeeDay, RateMode};
use horae_core::types::ProjectType;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportVisibility {
    Managers,
    ProjectMembers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeeMode {
    Single,
    Milestones,
    Monthly,
}

/// Stable field identities in creation rejections, independent of display text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectFormField {
    PaymentTerms,
    PurchaseOrder,
    Tax,
    SecondTaxName,
    SecondTax,
    Discount,
}

/// Keep raw values so autosaving does not discard an incomplete date or amount.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectForm {
    pub client_id: Option<Uuid>,
    pub name: String,
    pub code: String,
    pub starts_on: String,
    pub ends_on: String,
    pub tags: Vec<String>,
    /// None follows the selected client; Some is an explicit billing currency.
    pub currency: Option<String>,
    pub admin_notes: String,
    pub report_visibility: ReportVisibility,
    pub project_type: ProjectType,
    pub rate_mode: RateMode,
    pub project_rate: String,
    pub budget_mode: BudgetMode,
    pub budget_value: String,
    pub budget_alert: bool,
    pub budget_alert_at: String,
    pub budget_monthly: bool,
    pub budget_nonbillable: bool,
    pub fee_mode: FeeMode,
    pub fee_amount: String,
    pub monthly_day: MonthlyFeeDay,
    pub milestones: Vec<MilestoneInput>,
    pub tasks: Vec<ProjectTaskInput>,
    pub team: Vec<ProjectMemberInput>,
    pub invoice_defaults: InvoiceDefaultsInput,
}

impl Default for ProjectForm {
    fn default() -> Self {
        Self {
            client_id: None,
            name: String::new(),
            code: String::new(),
            starts_on: String::new(),
            ends_on: String::new(),
            tags: Vec::new(),
            currency: None,
            admin_notes: String::new(),
            report_visibility: ReportVisibility::Managers,
            project_type: ProjectType::TimeAndMaterials,
            rate_mode: RateMode::Person,
            project_rate: String::new(),
            budget_mode: BudgetMode::None,
            budget_value: String::new(),
            budget_alert: false,
            budget_alert_at: "80".into(),
            budget_monthly: false,
            budget_nonbillable: false,
            fee_mode: FeeMode::Single,
            fee_amount: String::new(),
            monthly_day: MonthlyFeeDay::First,
            milestones: Vec::new(),
            tasks: Vec::new(),
            team: Vec::new(),
            invoice_defaults: InvoiceDefaultsInput::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MilestoneInput {
    pub id: Uuid,
    pub name: String,
    pub due_on: String,
    pub amount: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TaskSource {
    Existing { task_id: Uuid },
    New { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TaskAccess {
    Everyone,
    Restricted { user_ids: Vec<Uuid> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectTaskInput {
    pub id: Uuid,
    pub source: TaskSource,
    pub billable: bool,
    pub rate: String,
    pub budget: String,
    pub access: TaskAccess,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectMemberInput {
    pub user_id: Uuid,
    pub manager: bool,
    pub billable_rate: String,
    pub cost_rate: String,
    pub budget: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvoiceDefaultsInput {
    pub terms_days: String,
    pub po_number: String,
    pub tax: String,
    pub second_tax: Option<SecondTaxInput>,
    pub discount: String,
}

impl Default for InvoiceDefaultsInput {
    fn default() -> Self {
        Self {
            terms_days: "30".into(),
            po_number: String::new(),
            tax: String::new(),
            second_tax: None,
            discount: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecondTaxInput {
    pub name: String,
    pub percentage: String,
}

/// Only returned to the draft's authorized creator, never in project lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectDraft {
    pub id: Uuid,
    pub revision: i64,
    pub saved_at: DateTime<Utc>,
    pub form: ProjectForm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftSaved {
    pub id: Uuid,
    pub revision: i64,
    pub saved_at: DateTime<Utc>,
}

/// Catalog options deliberately exclude addresses, identities and unrelated profile data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationClient {
    pub id: Uuid,
    pub name: String,
    pub currency: String,
    pub active: bool,
    pub default_rate_cents: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationTask {
    pub id: Uuid,
    pub name: String,
    pub billable: bool,
    pub default_rate_cents: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationPerson {
    pub id: Uuid,
    pub name: String,
    pub billable_rate_cents: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_rate_cents: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationOptions {
    pub organization_currency: String,
    pub can_edit_private_settings: bool,
    pub email_available: bool,
    pub clients: Vec<CreationClient>,
    pub tasks: Vec<CreationTask>,
    pub people: Vec<CreationPerson>,
    pub more_clients: bool,
    pub more_tasks: bool,
    pub more_people: bool,
    pub previous_code: Option<String>,
    pub suggested_code: Option<String>,
}

/// Selected active identities that may be outside the current catalog page.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationSelection {
    pub tasks: Vec<CreationTask>,
    pub people: Vec<CreationPerson>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogSearch {
    pub query: String,
    pub offset: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreationSearch {
    pub clients: CatalogSearch,
    pub tasks: CatalogSearch,
    pub people: CatalogSearch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_roundtrip_preserves_incomplete_input_without_demo_data() {
        let form = ProjectForm {
            project_rate: "12.".into(),
            starts_on: "2026-".into(),
            ..Default::default()
        };
        let decoded: ProjectForm =
            serde_json::from_value(serde_json::to_value(&form).unwrap()).unwrap();
        assert_eq!(decoded, form);
        assert!(form.client_id.is_none() && form.tasks.is_empty() && form.team.is_empty());
    }

    #[test]
    fn unknown_draft_fields_are_rejected() {
        let mut payload = serde_json::to_value(ProjectForm::default()).unwrap();
        payload["org_id"] = serde_json::json!(Uuid::now_v7());
        assert!(serde_json::from_value::<ProjectForm>(payload).is_err());
    }

    #[test]
    fn unauthorized_catalog_cost_is_absent_from_serialized_options() {
        let person = CreationPerson {
            id: Uuid::now_v7(),
            name: "Member".into(),
            billable_rate_cents: None,
            cost_rate_cents: None,
        };
        let payload = serde_json::to_value(person).unwrap();
        assert!(payload.get("cost_rate_cents").is_none());
    }
}
