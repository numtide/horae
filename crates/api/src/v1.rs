//! `/api/v1`, Horae's public API (spec 022). Every operation is registered
//! through `utoipa-axum`, so a route cannot exist without its description in
//! the OpenAPI document served at `/api/v1/openapi.json` (research R-1).
//!
//! Every operation but the document itself needs a bearer token; the session
//! cookie is never read (FR-010). Handlers translate: they call one shared
//! operation and convert its answer, and hold no rule of their own.

use std::sync::{Arc, OnceLock};

use axum::extract::{FromRef, FromRequestParts, Request, State};
use axum::http::StatusCode;
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use sqlx::PgPool;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::openapi::{
    ContentBuilder, HeaderBuilder, ObjectBuilder, Ref, RefOr, ResponseBuilder, Type,
};
use utoipa::{Modify, OpenApi, ToResponse};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::config::{ApiConfig, CursorKey};
use crate::notifications::budgets::check_project_budget;
use crate::ops::{EventSink, NoEvents};
use crate::problem::{self, Problem};
use crate::rate_limit::{self, RateLimiter};
use crate::state::AppState;
use crate::token::ApiCaller;

mod catalogue;
pub mod clients;
pub mod me;
pub mod people;
pub mod projects;
pub mod tasks;
#[cfg(test)]
pub(crate) mod testing;
pub mod time_entries;
pub mod timers;
pub mod types;

/// What `/api/v1` handlers share.
#[derive(Clone)]
pub struct V1State {
    pub pool: PgPool,
    pub limiter: Arc<RateLimiter>,
    /// Signs list cursors ([`crate::cursor`]).
    pub cursor_key: Arc<CursorKey>,
    /// Hears the events a write commits.
    pub events: Arc<dyn EventSink + Send + Sync>,
    /// Checks the budgets of the projects a write touched, as the web app
    /// does after its commit; `None` checks none.
    pub budgets: Option<&'static AppState>,
}

impl V1State {
    /// No events are dispatched and no budget is checked until
    /// [`V1State::with_app`].
    pub fn new(pool: PgPool, config: &ApiConfig) -> Self {
        Self {
            pool,
            limiter: Arc::new(RateLimiter::new(config.rate_limit)),
            cursor_key: Arc::new(config.cursor_key.clone()),
            events: Arc::new(NoEvents),
            budgets: None,
        }
    }

    /// The server's plugins hear writes, and its budgets are checked.
    pub fn with_app(self, app: &'static AppState) -> Self {
        Self {
            events: app.plugins.clone(),
            budgets: Some(app),
            ..self
        }
    }

    /// After a committed time change: the web app's budget checks, never
    /// blocking the answer.
    pub fn time_changed(&self, projects: impl IntoIterator<Item = uuid::Uuid>) {
        if let Some(app) = self.budgets {
            for project in projects {
                tokio::spawn(check_project_budget(app, project));
            }
        }
    }
}

impl FromRef<V1State> for PgPool {
    fn from_ref(state: &V1State) -> Self {
        state.pool.clone()
    }
}

impl FromRef<V1State> for Arc<RateLimiter> {
    fn from_ref(state: &V1State) -> Self {
        state.limiter.clone()
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Horae API",
        version = "1",
        description = "Horae's public API. Within v1 only backward-compatible changes are made: \
                       clients must ignore unknown response fields and treat an unknown error \
                       `code` as its status."
    ),
    servers((url = "/api/v1")),
    security(("token" = [])),
    components(schemas(Problem), responses(ProblemResponse)),
    modifiers(&BearerToken),
    tags(
        (name = "description", description = "This document"),
        (name = "identity", description = "Who a token acts as"),
        (name = "time entries", description = "Time spent, read and written as on the Timesheet"),
        (name = "timers", description = "Starting, stopping and restarting timers"),
        (name = "catalogue", description = "Projects, tasks, clients and people, read-only"),
    )
)]
struct ApiDoc;

struct BearerToken;

impl Modify for BearerToken {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "token",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .description(Some(
                        "A personal access token (horae_pat_…) created in personal settings.",
                    ))
                    .build(),
            ),
        );
    }
}

/// Every error answer: a [`Problem`], with `Retry-After` on 429.
pub struct ProblemResponse;

impl<'s> ToResponse<'s> for ProblemResponse {
    fn response() -> (&'s str, RefOr<utoipa::openapi::Response>) {
        let seconds = ObjectBuilder::new().schema_type(Type::Integer).build();
        let response = ResponseBuilder::new()
            .description("An error")
            .header(
                "Retry-After",
                HeaderBuilder::new()
                    .schema(seconds)
                    .description(Some("Seconds to wait (429 only)"))
                    .build(),
            )
            .content(
                problem::CONTENT_TYPE,
                ContentBuilder::new()
                    .schema(Some(Ref::from_schema_name("Problem")))
                    .build(),
            )
            .build();
        ("Problem", response.into())
    }
}

/// The OpenAPI document `utoipa` builds from the routes below.
#[utoipa::path(
    get,
    path = "/openapi.json",
    operation_id = "getOpenApi",
    tag = "description",
    security(()),
    responses((status = 200, description = "This document", content_type = "application/json"))
)]
async fn openapi_json() -> Json<&'static utoipa::openapi::OpenApi> {
    Json(document())
}

fn public_routes() -> OpenApiRouter<V1State> {
    OpenApiRouter::new().routes(routes!(openapi_json))
}

fn token_routes() -> OpenApiRouter<V1State> {
    OpenApiRouter::new()
        .routes(routes!(me::me))
        .routes(routes!(me::trackable))
        .routes(routes!(me::timer))
        .routes(routes!(time_entries::list, time_entries::create))
        .routes(routes!(
            time_entries::get,
            time_entries::update,
            time_entries::delete
        ))
        .routes(routes!(time_entries::stop))
        .routes(routes!(time_entries::restart))
        .routes(routes!(timers::start))
        .routes(routes!(projects::list))
        .routes(routes!(projects::get))
        .routes(routes!(tasks::list))
        .routes(routes!(tasks::get))
        .routes(routes!(clients::list))
        .routes(routes!(clients::get))
        .routes(routes!(people::list))
        .routes(routes!(people::get))
}

/// The published description of `/api/v1`, built once.
pub fn document() -> &'static utoipa::openapi::OpenApi {
    static DOCUMENT: OnceLock<utoipa::openapi::OpenApi> = OnceLock::new();
    DOCUMENT.get_or_init(|| {
        OpenApiRouter::<V1State>::with_openapi(ApiDoc::openapi())
            .merge(public_routes())
            .merge(token_routes())
            .split_for_parts()
            .1
    })
}

/// Every `/api/v1` route. Token routes check the token, then its request
/// limit.
pub fn router(state: V1State) -> Router {
    let (tokens, _) = token_routes().split_for_parts();
    let tokens = tokens
        .route_layer(from_fn_with_state(state.clone(), rate_limit::limit))
        .route_layer(from_fn_with_state(state.clone(), authenticate));
    let (public, _) = public_routes().split_for_parts();
    Router::new()
        .nest("/api/v1", tokens.merge(public))
        .with_state(state)
}

/// Puts the token's [`Caller`](crate::ops::Caller) in the request extensions,
/// or answers 401 `unauthenticated`.
async fn authenticate(State(state): State<V1State>, request: Request, next: Next) -> Response {
    let (mut parts, body) = request.into_parts();
    match ApiCaller::from_request_parts(&mut parts, &state).await {
        Ok(ApiCaller(caller)) => {
            parts.extensions.insert(caller);
            next.run(Request::from_parts(parts, body)).await
        }
        Err(StatusCode::UNAUTHORIZED) => Problem::unauthenticated().into_response(),
        Err(_) => Problem::internal().into_response(),
    }
}

#[cfg(test)]
mod tests;
