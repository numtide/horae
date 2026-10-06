use dioxus::prelude::ServerFnError;

use crate::models::people::PeopleCursor;
use crate::models::project_creation::{
    CreationOptions, CreationPerson, CreationSearch, ProjectEditorCatalogSearch,
    ProjectEditorContext,
};
use crate::models::project_people::{ProjectPeopleContext, ProjectPeopleQuery};
use crate::server_fns;

pub(super) async fn options(
    context: Option<ProjectEditorContext>,
    search: CreationSearch,
) -> Result<CreationOptions, ServerFnError> {
    let Some(context) = context else {
        return server_fns::project_creation_options(search).await;
    };
    let result = super::session::finish(
        server_fns::project_editor_catalog(
            context,
            ProjectEditorCatalogSearch {
                clients: search.clients,
                tasks: search.tasks,
            },
        )
        .await,
    )?;
    if result.context != context {
        return Err(super::session::report(super::session::changed()));
    }
    Ok(CreationOptions {
        organization_currency: result.organization_currency,
        email_available: result.email_available,
        clients: result.clients,
        tasks: result.tasks,
        more_clients: result.more_clients,
        more_tasks: result.more_tasks,
        people: Vec::new(),
        more_people: false,
        can_edit_private_settings: false,
        previous_code: None,
        suggested_code: None,
    })
}

pub(super) struct PeoplePage {
    pub people: Vec<CreationPerson>,
    pub more_people: bool,
    next_after: Option<PeopleCursor>,
}

pub(super) async fn people(
    context: Option<ProjectEditorContext>,
    query: String,
) -> Result<PeoplePage, ServerFnError> {
    people_page(context, query, None, 0).await
}

async fn people_page(
    context: Option<ProjectEditorContext>,
    query: String,
    after: Option<PeopleCursor>,
    offset: u32,
) -> Result<PeoplePage, ServerFnError> {
    if let Some(context) = context {
        let result = super::session::finish(
            server_fns::project_people(
                ProjectPeopleContext::Edit {
                    project_id: context.project_id,
                },
                ProjectPeopleQuery::Search { query, after },
            )
            .await,
        )?;
        if result.requester != context.requester {
            return Err(super::session::report(super::session::changed()));
        }
        Ok(PeoplePage {
            people: result
                .people
                .into_iter()
                .map(|person| CreationPerson {
                    id: person.id,
                    name: person.name,
                    // Candidate identity does not authorize profile/default rates.
                    billable_rate_cents: None,
                    cost_rate_cents: None,
                })
                .collect(),
            more_people: result.next_after.is_some(),
            next_after: result.next_after,
        })
    } else {
        let mut search = CreationSearch::default();
        search.people.query = query;
        search.people.offset = offset;
        let result = server_fns::project_creation_options(search).await?;
        Ok(PeoplePage {
            people: result.people,
            more_people: result.more_people,
            next_after: None,
        })
    }
}

pub(super) async fn all_people(
    context: Option<ProjectEditorContext>,
) -> Result<Vec<CreationPerson>, ServerFnError> {
    let mut all = Vec::new();
    let mut after = None;
    let mut offset = 0;
    loop {
        let result = people_page(context, String::new(), after, offset).await?;
        all.extend(result.people);
        if all.len() > 500 {
            return Err(ServerFnError::new(
                "A project can have at most 500 people. Select teammates individually.",
            ));
        }
        if !result.more_people {
            return Ok(all);
        }
        after = result.next_after;
        offset += 50;
    }
}
