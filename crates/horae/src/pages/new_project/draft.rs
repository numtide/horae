use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::project_creation::{DraftSaved, ProjectDraft, ProjectForm};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PendingSave {
    pub revision: i64,
    pub form: ProjectForm,
}

/// Keep the exact unacknowledged request: a lost response must be retried before
/// later edits, otherwise the server cannot recognize its idempotency match.
#[derive(Clone)]
pub(super) struct DraftState {
    pub id: Uuid,
    pub revision: i64,
    pub saved_at: Option<DateTime<Utc>>,
    pub saved: ProjectForm,
    pending: Option<PendingSave>,
}

impl DraftState {
    pub fn new(draft: Option<ProjectDraft>) -> Self {
        match draft {
            Some(draft) => Self {
                id: draft.id,
                revision: draft.revision,
                saved_at: Some(draft.saved_at),
                saved: draft.form,
                pending: None,
            },
            None => Self {
                id: Uuid::now_v7(),
                revision: 0,
                saved_at: None,
                saved: ProjectForm::default(),
                pending: None,
            },
        }
    }

    pub fn is_dirty(&self, form: &ProjectForm) -> bool {
        self.pending.is_some() || &self.saved != form
    }

    pub fn request(&mut self, form: &ProjectForm) -> PendingSave {
        self.pending
            .get_or_insert_with(|| PendingSave {
                revision: self.revision,
                form: form.clone(),
            })
            .clone()
    }

    pub fn acknowledge(&mut self, saved: DraftSaved) -> Result<(), &'static str> {
        let Some(request) = &self.pending else {
            return Err("No draft save is awaiting acknowledgement.");
        };
        if saved.id != self.id || request.revision.checked_add(1) != Some(saved.revision) {
            return Err("Unexpected draft revision. Reload the saved draft before continuing.");
        }
        self.saved = request.form.clone();
        self.revision = saved.revision;
        self.saved_at = Some(saved.saved_at);
        self.pending = None;
        Ok(())
    }

    /// A definite validation failure did not commit; a network error might have.
    pub fn reject_validation(&mut self) {
        self.pending = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project_creation::{DraftSaved, ProjectDraft, ProjectForm};
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn an_empty_form_is_not_reported_as_saved() {
        let state = DraftState::new(None);
        assert!(state.saved_at.is_none());
        assert_eq!(state.revision, 0);
        assert!(!state.is_dirty(&ProjectForm::default()));
    }

    #[test]
    fn a_retry_keeps_the_unacknowledged_payload_and_revision() {
        let mut state = DraftState::new(None);
        let first = ProjectForm {
            name: "First edit".into(),
            ..Default::default()
        };
        let mut latest = first.clone();
        latest.name = "Later edit".into();
        let request = state.request(&first);
        assert_eq!(state.request(&latest), request);
        state
            .acknowledge(DraftSaved {
                id: state.id,
                revision: 1,
                saved_at: Utc::now(),
            })
            .unwrap();
        assert!(state.is_dirty(&latest));
        assert!(!state.is_dirty(&first));
        assert_eq!(state.request(&latest).revision, 1);
        assert_eq!(state.request(&latest).form, latest);
    }

    #[test]
    fn a_mismatched_acknowledgement_does_not_lose_pending_input() {
        let mut state = DraftState::new(None);
        let form = ProjectForm {
            name: "Keep me".into(),
            ..Default::default()
        };
        let request = state.request(&form);
        for (id, revision) in [(Uuid::now_v7(), 1), (state.id, 9)] {
            assert!(
                state
                    .acknowledge(DraftSaved {
                        id,
                        revision,
                        saved_at: Utc::now()
                    })
                    .is_err()
            );
            assert_eq!(state.request(&form), request);
            assert_eq!(state.revision, 0);
        }
    }

    #[test]
    fn a_validation_rejection_allows_corrected_input_at_the_same_revision() {
        let mut state = DraftState::new(None);
        let bad = ProjectForm {
            name: "Invalid input".into(),
            ..Default::default()
        };
        state.request(&bad);
        state.reject_validation();
        let corrected = ProjectForm {
            name: "Corrected".into(),
            ..Default::default()
        };
        let request = state.request(&corrected);
        assert_eq!(request.form, corrected);
        assert_eq!(request.revision, 0);
    }

    #[test]
    fn resuming_preserves_the_server_identity_and_incomplete_fields() {
        let draft = ProjectDraft {
            id: Uuid::now_v7(),
            revision: 7,
            saved_at: Utc::now(),
            form: ProjectForm {
                project_rate: "12.".into(),
                ..Default::default()
            },
        };
        let state = DraftState::new(Some(draft.clone()));
        assert_eq!(state.id, draft.id);
        assert_eq!(state.revision, 7);
        assert_eq!(state.saved_at, Some(draft.saved_at));
        assert!(!state.is_dirty(&draft.form));
    }
}
