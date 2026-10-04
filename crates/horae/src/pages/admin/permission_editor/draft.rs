use horae_core::permissions::catalog::{
    BuiltInProfile, PERMISSION_CATALOG_VERSION, Permission, PermissionSelection,
};
use uuid::Uuid;

use crate::models::permission_editor::{
    PermissionEditor, ProfileAction, ProfileCommand, ProfileDraft, ProfilePreview,
};

#[derive(Clone, PartialEq)]
pub(super) struct DraftState {
    pub editor: PermissionEditor,
    pub selection: PermissionSelection,
    pub action: ProfileAction,
    pub preview: Option<ProfilePreview>,
    pub confirmed: bool,
    pub request: Option<ProfileCommand>,
}

impl DraftState {
    pub fn new(editor: PermissionEditor) -> Result<Self, String> {
        let selection = PermissionSelection::from_stored(
            PERMISSION_CATALOG_VERSION,
            &editor.permissions.grants,
        )
        .map_err(|_| "Could not load this person's permissions. Reload to try again.".to_owned())?;
        Ok(Self {
            editor,
            selection,
            action: ProfileAction::Edit,
            preview: None,
            confirmed: false,
            request: None,
        })
    }

    pub fn administrator(&self) -> bool {
        match self.action {
            ProfileAction::Edit => self.editor.permissions.is_administrator,
            ProfileAction::BuiltIn { profile } => profile == BuiltInProfile::Administrator,
            ProfileAction::Template { .. } => false,
        }
    }

    fn editable(&self) -> Result<(), String> {
        if self.request.is_some() {
            return Err("Resolve the previous save before making another change.".into());
        }
        Ok(())
    }

    pub fn invalidate_preview(&mut self) {
        self.preview = None;
        self.confirmed = false;
    }

    pub fn choose_builtin(&mut self, profile: BuiltInProfile) -> Result<(), String> {
        self.editable()?;
        self.action = ProfileAction::BuiltIn { profile };
        self.selection = profile.selection();
        self.invalidate_preview();
        Ok(())
    }

    pub fn choose_template(&mut self, id: Uuid) -> Result<(), String> {
        self.editable()?;
        let template = self
            .editor
            .templates
            .iter()
            .find(|template| template.id == id)
            .ok_or_else(|| "This profile is unavailable. Reload to try again.".to_owned())?;
        let selection =
            PermissionSelection::from_stored(PERMISSION_CATALOG_VERSION, &template.grants)
                .map_err(|_| "This profile could not be loaded. Reload to try again.".to_owned())?;
        self.action = ProfileAction::Template {
            id,
            expected_revision: template.revision,
        };
        self.selection = selection;
        self.invalidate_preview();
        Ok(())
    }

    pub fn toggle(&mut self, permission: Permission) -> Result<(), String> {
        self.editable()?;
        if self.administrator() {
            return Err("Choose a different profile to limit Administrator access.".into());
        }
        if self.selection.contains(permission) {
            self.selection
                .remove(permission)
                .map_err(|error| error.to_string())?;
        } else {
            self.selection.add(permission);
        }
        self.invalidate_preview();
        Ok(())
    }

    pub fn draft(&self) -> ProfileDraft {
        ProfileDraft {
            user_id: self.editor.user_id,
            expected_access_revision: self.editor.access_revision,
            expected_person_revision: self.editor.permissions.revision,
            action: self.action.clone(),
            grants: self.selection.iter().collect(),
        }
    }

    pub fn keep_project_access(&mut self) -> Result<(), String> {
        self.editable()?;
        if self
            .preview
            .as_ref()
            .is_none_or(|preview| preview.remove_projects.is_empty())
        {
            return Err("Review the project responsibilities to remove first.".into());
        }
        self.selection.add(Permission::ProjectWriteManaged);
        self.invalidate_preview();
        Ok(())
    }

    pub fn reviewed(&mut self, preview: ProfilePreview) -> Result<(), String> {
        self.editable()?;
        self.invalidate_preview();
        if preview.user_id != self.editor.user_id
            || preview.access_revision != self.editor.access_revision
            || preview.before != self.editor.permissions
            || preview.after.grants != self.draft().grants
        {
            return Err(
                "The preview no longer matches your changes. Reload and review again.".into(),
            );
        }
        self.preview = Some(preview);
        Ok(())
    }

    pub fn begin_save(&mut self) -> Option<ProfileCommand> {
        if let Some(command) = &self.request {
            return Some(command.clone());
        }
        let preview = self.preview.as_ref()?;
        if (!preview.remove_projects.is_empty() || !preview.remove_people.is_empty())
            && !self.confirmed
        {
            return None;
        }
        let draft = self.draft();
        let command = ProfileCommand {
            request_id: Uuid::now_v7(),
            expected_access_revision: draft.expected_access_revision,
            user_id: draft.user_id,
            expected_person_revision: draft.expected_person_revision,
            action: draft.action,
            grants: draft.grants,
            remove_projects: preview.remove_projects.iter().map(|item| item.id).collect(),
            remove_people: preview.remove_people.iter().map(|item| item.id).collect(),
        };
        self.request = Some(command.clone());
        Some(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::permission_editor::{
        PermissionSnapshot, ProfileSource, RelationshipRemoval,
    };

    fn editor() -> PermissionEditor {
        PermissionEditor {
            user_id: Uuid::now_v7(),
            name: "Person".into(),
            active: true,
            access_revision: 7,
            permissions: PermissionSnapshot {
                grants: BuiltInProfile::ProjectManager.selection().iter().collect(),
                is_administrator: false,
                source: ProfileSource::BuiltIn(BuiltInProfile::ProjectManager),
                revision: 3,
            },
            templates: vec![],
        }
    }

    fn preview(state: &DraftState) -> ProfilePreview {
        ProfilePreview {
            user_id: state.editor.user_id,
            access_revision: state.editor.access_revision,
            before: state.editor.permissions.clone(),
            after: PermissionSnapshot {
                grants: state.selection.iter().collect(),
                ..state.editor.permissions.clone()
            },
            changed: true,
            remove_projects: vec![],
            remove_people: vec![],
        }
    }

    #[test]
    fn load_keeps_exact_grants_and_explicit_edit_intent() {
        let loaded = editor();
        let state = DraftState::new(loaded.clone()).unwrap();
        assert_eq!(state.draft().grants, loaded.permissions.grants);
        assert_eq!(state.draft().action, ProfileAction::Edit);
        assert!(state.request.is_none());
    }

    #[test]
    fn malformed_loaded_grants_are_not_normalized() {
        let mut loaded = editor();
        loaded.permissions.grants.clear();
        assert!(DraftState::new(loaded).is_err());
    }

    #[test]
    fn editing_invalidates_preview_and_confirmation() {
        let mut state = DraftState::new(editor()).unwrap();
        state.reviewed(preview(&state)).unwrap();
        state.confirmed = true;
        state.toggle(Permission::ProjectReadManaged).unwrap();
        assert!(state.preview.is_none());
        assert!(!state.confirmed);
        assert!(!state.selection.contains(Permission::ProjectWriteManaged));
        assert!(state.begin_save().is_none());
    }

    #[test]
    fn joint_losses_need_confirmation_and_retry_preserves_exact_command() {
        let mut state = DraftState::new(editor()).unwrap();
        let mut effects = preview(&state);
        effects.remove_projects.push(RelationshipRemoval {
            id: Uuid::now_v7(),
            subject_id: Uuid::now_v7(),
            revision: 1,
        });
        effects.remove_people.push(RelationshipRemoval {
            id: Uuid::now_v7(),
            subject_id: Uuid::now_v7(),
            revision: 2,
        });
        state.reviewed(effects.clone()).unwrap();
        assert!(state.begin_save().is_none());
        state.confirmed = true;
        let command = state.begin_save().unwrap();
        assert_eq!(command.remove_projects, vec![effects.remove_projects[0].id]);
        assert_eq!(command.remove_people, vec![effects.remove_people[0].id]);
        assert_eq!(state.begin_save(), Some(command.clone()));
        assert!(state.toggle(Permission::ClientReadAll).is_err());
        assert_eq!(state.request, Some(command));
    }

    #[test]
    fn stale_or_mismatched_preview_cannot_be_confirmed() {
        for mismatch in 0..4 {
            let mut state = DraftState::new(editor()).unwrap();
            let mut effects = preview(&state);
            match mismatch {
                0 => effects.access_revision += 1,
                1 => effects.user_id = Uuid::now_v7(),
                2 => effects.before.revision += 1,
                _ => effects.after.grants.clear(),
            }
            assert!(state.reviewed(effects).is_err());
            assert!(state.begin_save().is_none());
        }
    }

    #[test]
    fn member_floor_and_explicit_administrator_are_not_customizable() {
        let mut state = DraftState::new(editor()).unwrap();
        assert!(state.toggle(Permission::TimeReadOwn).is_err());
        state.choose_builtin(BuiltInProfile::Administrator).unwrap();
        assert!(state.administrator());
        assert!(state.toggle(Permission::ClientReadAll).is_err());
        state.choose_builtin(BuiltInProfile::Member).unwrap();
        assert!(!state.administrator());
    }

    #[test]
    fn keeping_project_access_is_explicit_and_requires_new_effects() {
        let mut state = DraftState::new(editor()).unwrap();
        state.choose_builtin(BuiltInProfile::Member).unwrap();
        let mut effects = preview(&state);
        effects.remove_projects.push(RelationshipRemoval {
            id: Uuid::now_v7(),
            subject_id: Uuid::now_v7(),
            revision: 0,
        });
        state.reviewed(effects).unwrap();
        state.keep_project_access().unwrap();
        assert!(state.selection.contains(Permission::ProjectReadManaged));
        assert!(state.selection.contains(Permission::ProjectWriteManaged));
        assert!(!state.selection.contains(Permission::ProjectCreateAll));
        assert!(!state.selection.contains(Permission::PeopleReadManaged));
        assert!(state.begin_save().is_none());
        assert!(!state.confirmed);
    }

    #[test]
    fn all_grants_do_not_make_an_individual_an_administrator() {
        let mut loaded = editor();
        loaded.permissions.grants = Permission::ALL.to_vec();
        let state = DraftState::new(loaded).unwrap();
        assert!(!state.administrator());
    }
}
