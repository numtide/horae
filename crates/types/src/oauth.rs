//! What the OAuth approval page shows (spec 024, data-model *Page DTOs*).

use serde::{Deserialize, Serialize};

/// A pending authorization request, as the approval page names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentDetails {
    pub request_id: String,
    pub app_name: String,
    /// Horae's own app rather than one the workspace registered.
    pub built_in: bool,
    /// Scope names (`time:read`, …), in a fixed order.
    pub scopes: Vec<String>,
    /// The host, or the app scheme, Allow sends the person back to.
    pub return_to_display: String,
    pub person_name: String,
    pub organization_name: String,
}
