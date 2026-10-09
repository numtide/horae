use std::collections::HashSet;

use crate::models::project_creation::{
    ProjectEditorAccess, ProjectFieldAccess, ProjectForm, ProtectedProjectField,
};

pub(super) fn preserved_fields(
    access: Option<&ProjectEditorAccess>,
    before: &ProjectForm,
    form: &ProjectForm,
    edited: &HashSet<ProtectedProjectField>,
) -> Vec<ProtectedProjectField> {
    use ProtectedProjectField::*;
    let Some(access) = access else {
        return Vec::new();
    };
    let mut fields = Vec::new();
    for field in [ProjectRate, Fees, InvoiceDefaults, Budget, PrivateNotes] {
        if !edited.contains(&field) {
            fields.push(field);
        }
    }
    for task in &form.tasks {
        let field = TaskRate(task.id);
        if !edited.contains(&field)
            && (access.billable != ProjectFieldAccess::Editable
                || before.tasks.iter().any(|old| old.source == task.source))
        {
            fields.push(field);
        }
    }
    for person in &form.team {
        let retained = before.team.iter().any(|old| old.user_id == person.user_id);
        for (field, access) in [
            (PersonRate(person.user_id), access.billable),
            (CostRate(person.user_id), access.costs),
        ] {
            if !edited.contains(&field) && (retained || access != ProjectFieldAccess::Editable) {
                fields.push(field);
            }
        }
    }
    fields
}
