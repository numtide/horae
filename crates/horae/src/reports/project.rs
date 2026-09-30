//! Project work summaries share the dashboard's authorized projection.

use super::*;
use crate::models::project::ProjectActivityInterval;
use crate::render::project::{fits_project_pdf, render_project_pdf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectPdfParams {
    from: Option<String>,
    to: Option<String>,
}

impl ProjectPdfParams {
    fn interval(&self) -> Result<Option<ProjectActivityInterval>, StatusCode> {
        horae_core::project_activity::ActivityRange::parse_optional(
            self.from.as_deref(),
            self.to.as_deref(),
        )
        .map(|range| {
            range.map(|range| ProjectActivityInterval {
                from: range.from(),
                to: range.to(),
            })
        })
        .map_err(|_| StatusCode::BAD_REQUEST)
    }
}

fn projection_error(error: dioxus::prelude::ServerFnError) -> StatusCode {
    match error {
        dioxus::prelude::ServerFnError::ServerError { code, .. } => {
            StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn download_error(status: StatusCode) -> (StatusCode, &'static str) {
    let message = match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            "Sign in with report access before downloading this project summary."
        }
        StatusCode::NOT_FOUND => "Project not found or unavailable to this account.",
        StatusCode::BAD_REQUEST | StatusCode::PAYLOAD_TOO_LARGE => {
            "Project summary exceeds the supported scope or size. Choose a shorter period, or use the detailed CSV export. No partial PDF was produced."
        }
        StatusCode::SERVICE_UNAVAILABLE => {
            "Export capacity is busy. Return to the project and retry shortly."
        }
        StatusCode::GATEWAY_TIMEOUT => {
            "Export timed out. Return to the project and choose a shorter period."
        }
        _ => "Could not create the project summary. Return to the project and retry.",
    };
    (status, message)
}

pub(crate) async fn export_pdf(
    session: Session,
    Path(project_id): Path<uuid::Uuid>,
    Query(params): Query<ProjectPdfParams>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let org_id = require_manager(&session).await.map_err(download_error)?;
    let (viewer_id, _) = require_session(&session).await.map_err(download_error)?;
    let interval = params.interval().map_err(download_error)?;
    let permit = bounded::ExportPermit::acquire().map_err(download_error)?;
    let state = crate::state::global_state().await;
    let details =
        crate::server_fns::fetch_project_details(&state.db, org_id, viewer_id, project_id)
            .await
            .map_err(limits::database_error)
            .map_err(download_error)?
            .ok_or_else(|| download_error(StatusCode::NOT_FOUND))?;
    let data = crate::server_fns::breakdown::fetch_project_breakdown(
        &state.db, org_id, viewer_id, project_id, interval,
    )
    .await
    .map_err(projection_error)
    .map_err(download_error)?;
    let name = details
        .code
        .as_ref()
        .filter(|code| !code.is_empty())
        .map_or_else(
            || details.name.clone(),
            |code| format!("[{code}] {}", details.name),
        );
    if !fits_project_pdf(&name, &details.client_name, &data) {
        return Err(download_error(StatusCode::PAYLOAD_TOO_LARGE));
    }
    let bytes = permit
        .render(move || {
            render_project_pdf(&name, &details.client_name, &data).map_err(|error| {
                tracing::error!("Project summary rendering failed: {error}");
                StatusCode::INTERNAL_SERVER_ERROR
            })
        })
        .await
        .map_err(download_error)?;
    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/pdf".to_owned(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"project-{project_id}.pdf\""),
            ),
            (
                axum::http::header::CACHE_CONTROL,
                "private, no-store".to_owned(),
            ),
        ],
        bytes,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_period_rejects_missing_empty_reversed_and_unknown_scope() {
        for json in [
            r#"{"from":"2026-09-01"}"#,
            r#"{"from":"","to":""}"#,
            r#"{"from":"2026-09-30","to":"2026-09-01"}"#,
        ] {
            assert!(
                serde_json::from_str::<ProjectPdfParams>(json)
                    .unwrap()
                    .interval()
                    .is_err()
            );
        }
        assert!(serde_json::from_str::<ProjectPdfParams>(r#"{"task_id":"anything"}"#).is_err());
        assert!(
            serde_json::from_str::<ProjectPdfParams>("{}")
                .unwrap()
                .interval()
                .unwrap()
                .is_none()
        );
    }
}
