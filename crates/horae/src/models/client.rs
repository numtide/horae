use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Client {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub currency: String,
    pub address: Option<String>,
    pub tax_id: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

/// Catalog details plus projects whose progress the current viewer may read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientSummary {
    pub client: Client,
    pub active_projects: i64,
    pub total_projects: i64,
    pub project_currencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientDetails {
    pub client: Client,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing: Option<ClientBilling>,
}

/// An absent section means restricted access; an absent rate means not set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientBilling {
    pub default_rate_cents: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientProfile {
    pub name: String,
    pub currency: String,
    pub address: Option<String>,
    pub tax_id: Option<String>,
}

/// Currency and rate observed by the editor, checked against the locked row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientBillingSnapshot {
    pub currency: String,
    pub default_rate_cents: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "amount",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ClientRateChange {
    Keep,
    Replace(String),
    Clear,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientProfileEdit {
    pub profile: ClientProfile,
    pub rate_change: ClientRateChange,
    pub original: ClientBillingSnapshot,
}
