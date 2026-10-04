use super::*;
use crate::models::permission_editor::{PermissionSnapshot, RelationshipRemoval};

fn effects() -> ProfilePreview {
    let before = PermissionSnapshot {
        grants: BuiltInProfile::Administrator.selection().iter().collect(),
        is_administrator: true,
        source: ProfileSource::BuiltIn(BuiltInProfile::Administrator),
        revision: 88,
    };
    ProfilePreview {
        user_id: Uuid::now_v7(),
        access_revision: 887766,
        before,
        after: PermissionSnapshot {
            grants: BuiltInProfile::Member.selection().iter().collect(),
            is_administrator: false,
            source: ProfileSource::BuiltIn(BuiltInProfile::Member),
            revision: 89,
        },
        changed: true,
        remove_projects: vec![RelationshipRemoval {
            id: Uuid::now_v7(),
            subject_id: Uuid::now_v7(),
            revision: 22,
        }],
        remove_people: vec![RelationshipRemoval {
            id: Uuid::now_v7(),
            subject_id: Uuid::now_v7(),
            revision: 33,
        }],
    }
}

#[test]
fn review_explains_identity_loss_and_both_exact_management_sets() {
    let effects = effects();
    let html = dioxus::ssr::render_element(preview_content(&effects));
    for expected in [
        "no longer be an Administrator",
        "Remove: Create projects",
        "Membership and existing work will be preserved",
        "will not restore",
    ] {
        assert!(html.contains(expected), "{html}");
    }
    for removal in effects.remove_people.iter().chain(&effects.remove_projects) {
        assert!(html.contains(&removal.subject_id.to_string()), "{html}");
        assert!(!html.contains(&removal.id.to_string()), "{html}");
    }
    assert!(!html.contains("887766"), "{html}");
    assert!(!html.contains("style="), "{html}");
}

#[test]
fn no_op_preview_does_not_invent_permission_changes() {
    let mut effects = effects();
    effects.after = effects.before.clone();
    effects.changed = false;
    effects.remove_projects.clear();
    effects.remove_people.clear();
    let html = dioxus::ssr::render_element(preview_content(&effects));
    assert!(
        html.contains("No changes to the saved configuration"),
        "{html}"
    );
    assert!(!html.contains("Remove:"), "{html}");
    assert!(!html.contains("Allow:"), "{html}");
    assert!(!html.contains("responsibilities to remove"), "{html}");
}

#[test]
fn unknown_save_failure_is_not_treated_as_a_definite_rejection() {
    for code in [408, 429, 500, 502, 503, 504] {
        let error = ServerFnError::ServerError {
            code,
            message: "private storage context".into(),
            details: None,
        };
        assert!(!definite_rejection(&error));
        assert!(!rejection_message(&error).contains("private"));
    }
    assert!(!definite_rejection(&ServerFnError::new("lost response")));
}

#[test]
fn known_rejections_require_recovery_and_hide_authentication_diagnostics() {
    for code in [BAD_REQUEST, UNAUTHORIZED, FORBIDDEN, NOT_FOUND, CONFLICT] {
        let error = ServerFnError::ServerError {
            code,
            message: "private diagnostic".into(),
            details: None,
        };
        assert!(definite_rejection(&error));
        if access_denied(&error) {
            assert!(!rejection_message(&error).contains("private"));
        }
    }
}
