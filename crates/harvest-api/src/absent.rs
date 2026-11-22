//! Lists of kinds Horae does not have answer an empty page with the full
//! envelope, so a pipeline that syncs every stream keeps going (spec 021 D-2,
//! FR-043). Their records answer not found, which the router already does.
//! Invoice payments are nested under an invoice, so they arrive with invoices.

use super::*;

/// Each absent kind's path and the envelope key its records go under.
pub(super) const KINDS: [(&str, &str); 7] = [
    ("/expenses", "expenses"),
    ("/expense_categories", "expense_categories"),
    ("/estimates", "estimates"),
    ("/estimate_item_categories", "estimate_item_categories"),
    ("/contacts", "contacts"),
    ("/invoice_item_categories", "invoice_item_categories"),
    ("/roles", "roles"),
];

#[derive(Deserialize)]
pub(super) struct PageQuery {
    page: Option<i64>,
    per_page: Option<i64>,
}

/// The empty list of `kind`, for a caller who may use the API at all.
pub(super) fn list(kind: &'static str) -> MethodRouter<HarvestState> {
    get(
        move |_: AuthUser, list: ListRequest, Query(query): Query<PageQuery>| async move {
            let window = list.window(query.page, query.per_page, None)?;
            Ok::<_, ApiError>(Json(list.pages::<serde_json::Value>(
                kind,
                Vec::new(),
                &window,
                0,
            )))
        },
    )
}
