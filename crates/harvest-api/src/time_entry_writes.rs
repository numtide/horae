//! Time-entry writes (spec 021 step 3, FR-030–FR-032): create with hours,
//! with a start and end time, or as a running timer; update, delete, stop and
//! restart. Each runs the Timesheet's own operation (`ops::time::apply` and
//! `patch`) as the token's owner, so every rule, lock, refusal and event is
//! the web app's; this module only translates Harvest's parameters.

use axum::Json;
use axum::body::Bytes;
use axum::extract::{FromRequest, Path, Request, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::NaiveDate;
use horae_core::importers::harvest::convert::hours_to_minutes;
use serde::Serialize;
use serde_json::{Map, Value};
use uuid::Uuid;

use super::auth::AuthUser;
use super::errors::ApiError;
use super::ids::{self, Kind};
use super::{HarvestState, time_entry_by_id};
use crate::ops::time::{EntryPatch, TimesheetChange};
use crate::ops::{self, Actor, Channel, Done, OpError, Tx};
use horae_harvest::v2::compat::HarvestTimeEntry;
use horae_types::scoped_time::{TimesheetCommand, TimesheetEntryInput, TimesheetWriteContext};

/// A write's parameters from the query string and a JSON or form body
/// (FR-025); a body value wins over a query value of the same name. Query
/// and form values arrive as text, so every reader accepts text as well as
/// JSON numbers and booleans.
pub(crate) struct Params(Map<String, Value>);

impl<S: Send + Sync> FromRequest<S> for Params {
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, ApiError> {
        let mut params = Map::new();
        let query = request.uri().query().unwrap_or_default().as_bytes();
        for (key, value) in form_urlencoded::parse(query) {
            // Credentials are the extractor's (`auth`), never entry fields.
            if key != "access_token" && key != "account_id" {
                params.insert(key.into_owned(), Value::String(value.into_owned()));
            }
        }
        let form = request
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("application/x-www-form-urlencoded"));
        let body = Bytes::from_request(request, state)
            .await
            .map_err(|_| ApiError::unprocessable("The request body could not be read"))?;
        if form {
            for (key, value) in form_urlencoded::parse(&body) {
                params.insert(key.into_owned(), Value::String(value.into_owned()));
            }
        } else if !body.iter().all(u8::is_ascii_whitespace) {
            match serde_json::from_slice(&body) {
                Ok(Value::Object(fields)) => params.extend(fields),
                _ => return Err(ApiError::unprocessable("The body must be a JSON object")),
            }
        }
        Ok(Self(params))
    }
}

impl Params {
    /// A value as text; `null` reads as absent.
    fn text(&self, key: &str) -> Result<Option<String>, ApiError> {
        match self.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(text)) => Ok(Some(text.clone())),
            // Under serde_json's `arbitrary_precision` a number keeps the
            // digits it was sent with, so hours never pass through `f64`.
            Some(Value::Number(number)) => Ok(Some(number.to_string())),
            Some(_) => Err(ApiError::unprocessable(format!(
                "{key} must be a number or a string"
            ))),
        }
    }

    fn date(&self, key: &str) -> Result<Option<NaiveDate>, ApiError> {
        self.text(key)?
            .map(|text| {
                text.parse().map_err(|_| {
                    ApiError::unprocessable(format!("{key} must be a YYYY-MM-DD date"))
                })
            })
            .transpose()
    }

    /// Absent keeps the notes; `null` clears them.
    fn notes(&self) -> Result<Option<Option<String>>, ApiError> {
        match self.0.get("notes") {
            None => Ok(None),
            Some(Value::Null) => Ok(Some(None)),
            Some(Value::String(notes)) => Ok(Some(Some(notes.clone()))),
            Some(_) => Err(ApiError::unprocessable("notes must be a string")),
        }
    }

    /// The record an id parameter names. A number no record of `kind` has
    /// names nothing, and the operation refuses it as the web app would.
    async fn record(
        &self,
        state: &HarvestState,
        user: &AuthUser,
        kind: Kind,
        key: &str,
    ) -> Result<Option<Uuid>, ApiError> {
        ids::filter(&state.0.pool, user.org_id, kind, self.text(key)?.as_deref()).await
    }

    fn times(&self) -> Result<Times, ApiError> {
        let clock = |key: &str| -> Result<Option<i32>, ApiError> {
            self.text(key)?
                .map(|text| {
                    horae_core::time_of_day::parse(&text)
                        .map(i32::from)
                        .ok_or_else(|| {
                            ApiError::unprocessable(format!("{key} must be a time of day"))
                        })
                })
                .transpose()
        };
        Ok(Times {
            minutes: self.text("hours")?.as_deref().map(minutes).transpose()?,
            started: clock("started_time")?,
            ended: clock("ended_time")?,
        })
    }
}

/// Decimal hours as whole minutes, rounded half up with integer arithmetic
/// (FR-020): `0.01` is 1 minute, `1.255` is 75 and `0.333` is 20.
fn minutes(hours: &str) -> Result<i32, ApiError> {
    let invalid = || ApiError::unprocessable("hours must be a non-negative decimal number");
    let minutes = hours_to_minutes(hours).map_err(|_| invalid())?;
    i32::try_from(minutes).map_err(|_| invalid())
}

/// What a request says about an entry's time, in minutes since midnight for
/// the clock times.
struct Times {
    minutes: Option<i32>,
    started: Option<i32>,
    ended: Option<i32>,
}

impl Times {
    /// The minutes and start this sets, given the entry's current start. A
    /// start and an end give the minutes between them, which the operation
    /// refuses when negative (an end before the start, or past midnight);
    /// otherwise `hours` gives the minutes and `started_time` the start.
    fn resolve(&self, start: Option<i32>) -> Result<(Option<i32>, Option<i32>), OpError> {
        match self.ended {
            Some(ended) => {
                let started = self.started.or(start).ok_or_else(|| {
                    OpError::invalid("started_time", "started_time is required with ended_time")
                })?;
                Ok((Some(ended - started), Some(started)))
            }
            None => Ok((self.minutes, self.started)),
        }
    }
}

/// A written entry as Harvest returns it. `horae_notice` explains a stop the
/// daily limit cut short; clients that do not know it ignore it.
#[derive(Serialize)]
pub(super) struct Written {
    #[serde(flatten)]
    entry: HarvestTimeEntry,
    #[serde(skip_serializing_if = "Option::is_none")]
    horae_notice: Option<String>,
}

/// Whose time a write changes: a person (the token's owner when absent), or
/// the person whose entry it names.
enum Subject {
    Person(Option<Uuid>),
    Entry(Uuid),
}

enum Change {
    Command(TimesheetCommand),
    Patch(Uuid, Times, EntryPatch),
}

/// Runs one Timesheet write as the token's owner and commits it. A request
/// authenticated only by the session cookie may not write (Principle IV).
async fn write(
    state: &HarvestState,
    user: &AuthUser,
    subject: Subject,
    change: Change,
) -> Result<TimesheetChange, ApiError> {
    if matches!(user.channel, Channel::Session) {
        return Err(ApiError::unauthorized(
            "Writes need an access token; the session cookie only reads",
        ));
    }
    let mut write = ops::begin(&state.0.pool, user.caller()).await?;
    let result = apply(&mut write.tx, &write.actor, subject, change).await;
    let mut change = write.commit(result, &*state.0.events).await?;
    state.0.time_changed(std::mem::take(&mut change.projects));
    Ok(change)
}

async fn apply(
    tx: &mut Tx,
    actor: &Actor,
    subject: Subject,
    change: Change,
) -> Result<Done<TimesheetChange>, OpError> {
    let (subject_id, start) = match subject {
        // An entry the actor may not read is not found, as in the web app.
        Subject::Entry(id) => {
            let entry = ops::time_reads::get(tx, actor, id).await?;
            (entry.user_id, entry.start_minute)
        }
        Subject::Person(person) => (person.unwrap_or(actor.id()), None),
    };
    // No page stands between the token and the write: the requester is the
    // actor itself.
    let context = TimesheetWriteContext {
        expected_requester: actor.requester(),
        subject_id,
    };
    match change {
        Change::Command(command) => ops::time::apply(tx, actor, &context, command).await,
        Change::Patch(id, times, mut patch) => {
            let (minutes, start) = times.resolve(start)?;
            patch.minutes = minutes;
            patch.start_minute = start.map(Some);
            ops::time::patch(tx, actor, &context, id, patch).await
        }
    }
}

/// The entry a write saved, read back through the listing's scope and shape.
async fn written(
    state: &HarvestState,
    user: &AuthUser,
    change: TimesheetChange,
) -> Result<Json<Written>, ApiError> {
    let entry = change
        .entry
        .ok_or_else(|| ApiError::internal("A Timesheet write answered no entry"))?;
    let Json(entry) = time_entry_by_id(&state.0.pool, user, entry.id).await?;
    Ok(Json(Written {
        entry,
        horae_notice: change.notice,
    }))
}

/// `POST /time_entries`: with `hours`, or `started_time` and `ended_time`, an
/// entry of that length; with neither, a running timer on the person's today,
/// which stops their running one (D-6).
pub(super) async fn create(
    user: AuthUser,
    State(state): State<HarvestState>,
    params: Params,
) -> Result<Response, ApiError> {
    let person = params.record(&state, &user, Kind::User, "user_id").await?;
    let project_id = required(
        params
            .record(&state, &user, Kind::Project, "project_id")
            .await?,
        "project_id",
    )?;
    let task_id = required(
        params.record(&state, &user, Kind::Task, "task_id").await?,
        "task_id",
    )?;
    let spent_date = required(params.date("spent_date")?, "spent_date")?;
    let notes = params.notes()?.flatten();
    let times = params.times()?;
    let command = match times.resolve(None).map_err(ApiError::from)? {
        // Horae files a timer on the person's own today (D9), whatever day
        // the client names.
        (None, None) => TimesheetCommand::StartTimer {
            project_id,
            task_id,
            notes,
        },
        (Some(minutes), start_minute) => TimesheetCommand::Create {
            entry: TimesheetEntryInput {
                project_id,
                task_id,
                spent_date,
                minutes,
                notes,
                // The task's eligibility still decides, as in the web app.
                billable: true,
                start_minute,
            },
        },
        (None, Some(_)) => {
            return Err(ApiError::unprocessable(
                "started_time needs ended_time or hours",
            ));
        }
    };
    let change = write(
        &state,
        &user,
        Subject::Person(person),
        Change::Command(command),
    )
    .await?;
    Ok((StatusCode::CREATED, written(&state, &user, change).await?).into_response())
}

/// `PATCH /time_entries/{id}`: the named fields change, the rest keep their
/// values. `ended_time` alone keeps the entry's start.
pub(super) async fn update(
    user: AuthUser,
    State(state): State<HarvestState>,
    Path(id): Path<String>,
    params: Params,
) -> Result<Json<Written>, ApiError> {
    let id = ids::path(&state.0.pool, user.org_id, Kind::TimeEntry, &id).await?;
    let patch = EntryPatch {
        project_id: params
            .record(&state, &user, Kind::Project, "project_id")
            .await?,
        task_id: params.record(&state, &user, Kind::Task, "task_id").await?,
        spent_date: params.date("spent_date")?,
        notes: params.notes()?,
        ..EntryPatch::default()
    };
    let change = Change::Patch(id, params.times()?, patch);
    let change = write(&state, &user, Subject::Entry(id), change).await?;
    written(&state, &user, change).await
}

/// `DELETE /time_entries/{id}`: 200 with an empty body, as on Harvest.
pub(super) async fn delete(
    user: AuthUser,
    State(state): State<HarvestState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = ids::path(&state.0.pool, user.org_id, Kind::TimeEntry, &id).await?;
    let command = TimesheetCommand::Delete {
        entry_ids: vec![id],
    };
    write(&state, &user, Subject::Entry(id), Change::Command(command)).await?;
    Ok(StatusCode::OK)
}

/// `PATCH /time_entries/{id}/stop`.
pub(super) async fn stop(
    user: AuthUser,
    State(state): State<HarvestState>,
    Path(id): Path<String>,
) -> Result<Json<Written>, ApiError> {
    let id = ids::path(&state.0.pool, user.org_id, Kind::TimeEntry, &id).await?;
    let command = TimesheetCommand::StopTimer { entry_id: id };
    let change = write(&state, &user, Subject::Entry(id), Change::Command(command)).await?;
    written(&state, &user, change).await
}

/// `PATCH /time_entries/{id}/restart`: today's entries only (409 otherwise);
/// the person's running timer stops first (D-6).
pub(super) async fn restart(
    user: AuthUser,
    State(state): State<HarvestState>,
    Path(id): Path<String>,
) -> Result<Json<Written>, ApiError> {
    let id = ids::path(&state.0.pool, user.org_id, Kind::TimeEntry, &id).await?;
    let command = TimesheetCommand::RestartTimer { entry_id: id };
    let change = write(&state, &user, Subject::Entry(id), Change::Command(command)).await?;
    written(&state, &user, change).await
}

fn required<T>(value: Option<T>, key: &str) -> Result<T, ApiError> {
    value.ok_or_else(|| ApiError::unprocessable(format!("{key} is required")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hours_round_half_up_to_whole_minutes() {
        for (hours, expected) in [
            ("0.01", 1),
            ("1.255", 75),
            ("0.333", 20),
            ("0.33", 20),
            ("1.5", 90),
            ("0", 0),
            ("0.0083", 0),
            ("0.00834", 1),
        ] {
            assert_eq!(minutes(hours).unwrap(), expected, "{hours}");
        }
    }

    #[test]
    fn hours_refuse_negative_malformed_and_huge_values() {
        for hours in ["-1", "1e2", "abc", "", "99999999999"] {
            assert_eq!(
                minutes(hours).unwrap_err().status,
                StatusCode::UNPROCESSABLE_ENTITY,
                "{hours}"
            );
        }
    }

    fn times(minutes: Option<i32>, started: Option<i32>, ended: Option<i32>) -> Times {
        Times {
            minutes,
            started,
            ended,
        }
    }

    #[test]
    fn a_start_and_an_end_give_the_minutes_between_them() {
        let span = times(Some(30), Some(540), Some(630)).resolve(None).unwrap();
        assert_eq!(span, (Some(90), Some(540)));
    }

    #[test]
    fn an_end_before_the_start_gives_negative_minutes_for_the_operation_to_refuse() {
        let span = times(None, Some(1380), Some(60)).resolve(None).unwrap();
        assert_eq!(span, (Some(-1320), Some(1380)));
    }

    #[test]
    fn an_end_alone_keeps_the_entrys_start_or_is_refused() {
        assert_eq!(
            times(None, None, Some(600)).resolve(Some(540)).unwrap(),
            (Some(60), Some(540))
        );
        assert!(times(None, None, Some(600)).resolve(None).is_err());
    }

    #[test]
    fn without_an_end_hours_and_start_pass_through() {
        assert_eq!(
            times(Some(45), Some(540), None).resolve(Some(0)).unwrap(),
            (Some(45), Some(540))
        );
        assert_eq!(times(None, None, None).resolve(None).unwrap(), (None, None));
    }
}
