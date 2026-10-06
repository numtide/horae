//! Historical relationship facts retained by permission-change receipts.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemovedRelationship {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub revision: i64,
}
