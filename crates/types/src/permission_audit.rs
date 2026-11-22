//! Historical permission-change projection. Never use this DTO as current authority.

use std::fmt;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use super::permission_editor::PermissionRequester;

/// An exclusive descending ordering bound, not a receipt lookup or authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditCursor {
    pub created_at: DateTime<Utc>,
    pub id: Uuid,
}

/// Which history a page shows. Every field narrows it; the default is all of it.
/// It is also the audit log's URL query (`?kind=person&person=<id>&period=30d`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditFilter {
    #[serde(default)]
    pub kind: Option<AuditKind>,
    /// Changes made by this person or to them: their permissions, their
    /// management, or a profile they were detached from.
    #[serde(default)]
    pub person: Option<Uuid>,
    #[serde(default)]
    pub period: Option<AuditPeriod>,
}

impl AuditFilter {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    pub fn person(person: Uuid) -> Self {
        Self {
            person: Some(person),
            ..Self::default()
        }
    }
}

/// Reads the URL query leniently: a value it does not know is left out, so an
/// edited address shows a wider view instead of an error.
impl From<&str> for AuditFilter {
    fn from(query: &str) -> Self {
        let mut filter = Self::default();
        for (key, value) in query.split('&').filter_map(|pair| pair.split_once('=')) {
            match key {
                "kind" => filter.kind = AuditKind::parse(value),
                "person" => filter.person = Uuid::parse_str(value).ok(),
                "period" => filter.period = AuditPeriod::parse(value),
                _ => {}
            }
        }
        filter
    }
}

impl fmt::Display for AuditFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts = [
            self.kind.map(|kind| format!("kind={}", kind.as_str())),
            self.person.map(|person| format!("person={person}")),
            self.period
                .map(|period| format!("period={}", period.as_str())),
        ];
        let query: Vec<String> = parts.into_iter().flatten().collect();
        f.write_str(&query.join("&"))
    }
}

/// What a receipt changed: a person's permissions, a custom profile, or a
/// project's managers. The names are the screen's, so they cross over with
/// the receipt's: `Person` is [`HistoricalAudit::Profile`] (a person's profile
/// change) and `Profile` is [`HistoricalAudit::Template`] (a custom profile).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditKind {
    Person,
    Profile,
    Managers,
}

impl AuditKind {
    pub const ALL: [Self; 3] = [Self::Person, Self::Profile, Self::Managers];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Profile => "profile",
            Self::Managers => "managers",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

/// How far back a page reaches, counted from now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditPeriod {
    #[serde(rename = "7d")]
    Days7,
    #[serde(rename = "30d")]
    Days30,
    #[serde(rename = "90d")]
    Days90,
    #[serde(rename = "365d")]
    Days365,
}

impl AuditPeriod {
    pub const ALL: [Self; 4] = [Self::Days7, Self::Days30, Self::Days90, Self::Days365];

    pub fn days(self) -> i32 {
        match self {
            Self::Days7 => 7,
            Self::Days30 => 30,
            Self::Days90 => 90,
            Self::Days365 => 365,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Days7 => "7d",
            Self::Days30 => "30d",
            Self::Days90 => "90d",
            Self::Days365 => "365d",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|period| period.as_str() == value)
    }
}

/// A bounded historical page admitted under the requester's current authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditPage {
    pub requester: PermissionRequester,
    pub entries: Vec<AuditEntry>,
    pub next_after: Option<AuditCursor>,
    /// The viewer's zone, in which `today` and each entry's `local` time are.
    pub time_zone: String,
    pub today: NaiveDate,
    /// Everyone in the organization, archived people included: the names the
    /// entries refer to and the choices of the person filter. Only the first
    /// page carries them; a page read after a cursor leaves this empty.
    pub people: Vec<AuditName>,
    /// The projects and custom profiles the entries refer to and that still exist.
    pub projects: Vec<AuditName>,
    pub profiles: Vec<AuditName>,
}

impl AuditPage {
    pub fn person_name(&self, id: Uuid) -> Option<&str> {
        find_name(&self.people, id)
    }

    pub fn project_name(&self, id: Uuid) -> Option<&str> {
        find_name(&self.projects, id)
    }

    pub fn profile_name(&self, id: Uuid) -> Option<&str> {
        find_name(&self.profiles, id)
    }
}

fn find_name(names: &[AuditName], id: Uuid) -> Option<&str> {
    names
        .iter()
        .find(|name| name.id == id)
        .map(|name| name.name.as_str())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditName {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuditPrincipal {
    User {
        user_id: Uuid,
    },
    Operator {
        invocation_id: String,
        command: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub actor: AuditPrincipal,
    pub created_at: DateTime<Utc>,
    /// `created_at` in the viewer's zone, which groups the entries by day.
    pub local: NaiveDateTime,
    pub audit: HistoricalAudit,
}

impl AuditEntry {
    pub fn kind(&self) -> AuditKind {
        match self.audit {
            HistoricalAudit::Profile(_) => AuditKind::Person,
            HistoricalAudit::Template(_) => AuditKind::Profile,
            HistoricalAudit::ProjectManagers(_) => AuditKind::Managers,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "details", rename_all = "snake_case")]
pub enum HistoricalAudit {
    Template(TemplateAudit),
    Profile(ProfileAudit),
    ProjectManagers(ProjectAudit),
}

// These are versioned historical wire types, not deserializable authority models.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateAudit {
    pub previous_access_revision: i64,
    pub access_revision: i64,
    #[serde(deserialize_with = "required_option")]
    pub before: Option<TemplateSnapshot>,
    #[serde(deserialize_with = "required_option")]
    pub after: Option<TemplateSnapshot>,
    pub detached_people: Vec<DetachedPerson>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateSnapshot {
    pub id: Uuid,
    pub name: String,
    pub catalog_version: i32,
    pub grants: Vec<Permission>,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetachedPerson {
    pub user_id: Uuid,
    pub catalog_version: i32,
    pub grants: Vec<Permission>,
    pub is_administrator: bool,
    pub previous_template_id: Uuid,
    pub previous_applied_revision: i64,
    pub previous_revision: i64,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileAudit {
    pub user_id: Uuid,
    pub previous_access_revision: i64,
    pub access_revision: i64,
    #[serde(deserialize_with = "required_option")]
    pub change: Option<ProfileChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileChange {
    pub catalog_version: u32,
    pub before: PersonSnapshot,
    pub after: PersonSnapshot,
    pub removed_projects: Vec<RemovedRelationship>,
    pub removed_people: Vec<RemovedRelationship>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonSnapshot {
    pub grants: Vec<Permission>,
    pub is_administrator: bool,
    pub source: HistoricalSource,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum HistoricalSource {
    BuiltIn(BuiltInProfile),
    Template { id: Uuid, applied_revision: i64 },
    Individual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemovedRelationship {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectAudit {
    pub project_id: Uuid,
    pub previous_access_revision: i64,
    pub access_revision: i64,
    #[serde(deserialize_with = "required_option")]
    pub change: Option<ProjectChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectChange {
    pub added: Vec<Designation>,
    pub removed: Vec<Designation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Designation {
    pub id: Uuid,
    pub manager_id: Uuid,
    pub revision: i64,
}

fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    // A missing field must not be treated as an explicit unchanged outcome.
    Option::<T>::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filter_survives_the_url() {
        let filter = AuditFilter {
            kind: Some(AuditKind::Managers),
            person: Some(Uuid::now_v7()),
            period: Some(AuditPeriod::Days30),
        };
        assert_eq!(AuditFilter::from(filter.to_string().as_str()), filter);
        assert_eq!(AuditFilter::default().to_string(), "");
    }

    #[test]
    fn unknown_query_values_widen_the_filter() {
        let filter = AuditFilter::from("kind=everything&person=nobody&period=30d&page=2");
        assert_eq!(
            filter,
            AuditFilter {
                period: Some(AuditPeriod::Days30),
                ..AuditFilter::default()
            }
        );
    }

    #[test]
    fn the_wire_rejects_an_unknown_kind_or_period() {
        assert!(serde_json::from_str::<AuditFilter>(r#"{"kind":"everything"}"#).is_err());
        assert!(serde_json::from_str::<AuditFilter>(r#"{"period":"2d"}"#).is_err());
        assert_eq!(
            serde_json::from_str::<AuditFilter>(r#"{"period":"7d"}"#).unwrap(),
            AuditFilter {
                period: Some(AuditPeriod::Days7),
                ..AuditFilter::default()
            }
        );
    }
}
