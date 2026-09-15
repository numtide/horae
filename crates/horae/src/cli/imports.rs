//! Network-only administration of the existing durable Harvest job surface.

use std::{cell::Cell, io::Write, time::Duration};

use horae_core::importers::harvest::types::{ImportMode, ImportReport, SyncScope};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::{Commands, ImportAction, JobAction, RemoteOptions, WaitOptions};
use crate::models::JobStatus;

#[cfg(test)]
mod tests;
mod transport;

#[derive(Debug, Serialize)]
pub(super) struct CliError {
    #[serde(skip)]
    code: u8,
    category: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    http_status: Option<u16>,
}

impl CliError {
    fn configuration(message: &str) -> Self {
        Self {
            code: 2,
            category: "configuration",
            message: message.into(),
            http_status: None,
        }
    }

    fn failure(category: &'static str, message: &str) -> Self {
        Self {
            code: 1,
            category,
            message: message.into(),
            http_status: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct CommandResult {
    version: u8,
    operation: &'static str,
    outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    job_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<CliError>,
    #[serde(skip)]
    code: u8,
}

struct Observation {
    job_id: Cell<Option<Uuid>>,
    request_id: Option<Uuid>,
}

pub fn run(command: &Commands, options: &RemoteOptions) -> u8 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(_) => {
            return emit(
                &failure_result(
                    "initialize",
                    CliError::configuration("Cannot initialize runtime"),
                ),
                options.json,
            );
        }
    };
    let result = runtime.block_on(run_async(command, options));
    emit(&result, options.json)
}

pub fn argument_error() -> u8 {
    let mut error = CliError::configuration("Invalid command arguments; use --help for usage");
    error.category = "arguments";
    emit(&failure_result("parse", error), true)
}

fn failure_result(operation: &'static str, error: CliError) -> CommandResult {
    CommandResult {
        version: 1,
        operation,
        outcome: "error",
        job_id: None,
        request_id: None,
        data: None,
        code: error.code,
        error: Some(error),
    }
}

async fn run_async(command: &Commands, options: &RemoteOptions) -> CommandResult {
    let operation = match command {
        Commands::Import {
            action: ImportAction::HarvestApi { .. },
        } => "import.harvest-api",
        Commands::Import { .. } => "import.harvest-csv",
        Commands::Jobs { action } => match action {
            JobAction::Status { .. } => "jobs.status",
            JobAction::List { .. } => "jobs.list",
            JobAction::Report { .. } => "jobs.report",
            JobAction::Errors { .. } => "jobs.errors",
            JobAction::Cancel { .. } => "jobs.cancel",
            JobAction::Retry { .. } => "jobs.retry",
            JobAction::Wait { .. } => "jobs.wait",
        },
        _ => return failure_result("dispatch", CliError::configuration("Not a remote command")),
    };
    let request_id = match command {
        Commands::Import { action } => Some(match action {
            ImportAction::HarvestApi { options, .. } | ImportAction::HarvestCsv { options, .. } => {
                options.request_id.unwrap_or_else(Uuid::now_v7)
            }
        }),
        _ => None,
    };
    let job_id = match command {
        Commands::Jobs { action } => match action {
            JobAction::Status { job_id }
            | JobAction::Report { job_id }
            | JobAction::Errors { job_id, .. }
            | JobAction::Cancel { job_id }
            | JobAction::Retry { job_id, .. }
            | JobAction::Wait { job_id, .. } => Some(*job_id),
            JobAction::List { .. } => None,
        },
        _ => None,
    };
    let observation = Observation {
        job_id: Cell::new(job_id),
        request_id,
    };
    let result = async {
        let path = options.session_file.as_deref().ok_or_else(|| CliError::configuration("Select --session-file or HORAE_SESSION_FILE"))?;
        let transport = transport::Transport::load(path)?;
        if let Some(id) = request_id {
            if id.get_version_num() != 7 {
                return Err(CliError::configuration("Request ID must be a UUIDv7"));
            }
            eprintln!("Request ID: {id}");
        }
        tokio::select! {
            result = execute(command, &transport, &observation) => result,
            signal = tokio::signal::ctrl_c() => {
                if signal.is_err() {
                    Err(CliError::failure("signal", "Cannot observe interrupt signal"))
                } else if request_id.is_some() && observation.job_id.get().is_none() {
                    Err(CliError { code: 6, category: "indeterminate_submission", message: "Submission interrupted before acknowledgement; resubmit identical input with the same request ID".into(), http_status: None })
                } else {
                    Err(CliError { code: 130, category: "interrupted", message: "Observation interrupted; job was not cancelled".into(), http_status: None })
                }
            }
        }
    }.await;
    let mut result = match result {
        Ok((outcome, data, code)) => CommandResult {
            version: 1,
            operation,
            outcome,
            job_id: None,
            request_id,
            data: Some(data),
            error: None,
            code,
        },
        Err(error) => failure_result(operation, error),
    };
    result.job_id = observation.job_id.get();
    result.request_id = observation.request_id;
    result
}

async fn execute(
    command: &Commands,
    client: &transport::Transport,
    observation: &Observation,
) -> Result<(&'static str, Value, u8), CliError> {
    match command {
        Commands::Import { action } => {
            let key = observation
                .request_id
                .ok_or_else(|| CliError::configuration("Missing request identity"))?;
            let (value, options) = match action {
                ImportAction::HarvestApi { full, options, .. } => {
                    let mode = if options.dry_run {
                        ImportMode::DryRun
                    } else {
                        ImportMode::Commit
                    };
                    let sync = if *full {
                        SyncScope::Full
                    } else {
                        SyncScope::Incremental
                    };
                    (
                        client
                            .post(
                                "/api/import/harvest/start",
                                &json!({"mode":mode,"sync":sync}),
                                Some(key),
                            )
                            .await?,
                        &options.wait,
                    )
                }
                ImportAction::HarvestCsv { file, options } => {
                    let mode = if options.dry_run {
                        ImportMode::DryRun
                    } else {
                        ImportMode::Commit
                    };
                    (
                        client
                            .csv(&format!("/api/import/harvest/csv-job/{mode}"), file, key)
                            .await?,
                        &options.wait,
                    )
                }
            };
            let job = decode_job(value).map_err(|_| CliError { code: 6, category: "indeterminate_submission", message: "Invalid submission acknowledgement; resubmit identical input with the same request ID".into(), http_status: None })?;
            observation.job_id.set(Some(job.id));
            finish_submission(client, job, options).await
        }
        Commands::Jobs { action } => match action {
            JobAction::List { before, limit } => {
                let value = client
                    .post(
                        "/api/import/harvest/history",
                        &json!({"before":before,"limit":limit}),
                        None,
                    )
                    .await?;
                let jobs: Vec<JobStatus> = serde_json::from_value(value.clone())
                    .map_err(|_| CliError::failure("protocol", "Invalid job history"))?;
                if jobs.len() > usize::from(*limit) {
                    return Err(CliError::failure(
                        "protocol",
                        "Job history exceeds requested limit",
                    ));
                }
                Ok(("observed", value, 0))
            }
            JobAction::Errors {
                job_id,
                output,
                force,
            } => {
                let bytes = client
                    .download(
                        &format!("/api/import/harvest/jobs/{job_id}/errors"),
                        output,
                        *force,
                    )
                    .await?;
                Ok(("downloaded", json!({"output":output,"bytes":bytes}), 0))
            }
            JobAction::Wait { job_id, timeout } => wait(client, *job_id, *timeout).await,
            JobAction::Retry { job_id, options } => {
                let job = decode_job(
                    client
                        .post("/api/import/harvest/retry", &json!({"job_id":job_id}), None)
                        .await?,
                )?;
                ensure_job_id(&job, *job_id)?;
                finish_submission(client, job, options).await
            }
            JobAction::Cancel { job_id } => {
                let job = decode_job(
                    client
                        .post(
                            "/api/import/harvest/cancel",
                            &json!({"job_id":job_id}),
                            None,
                        )
                        .await?,
                )?;
                ensure_job_id(&job, *job_id)?;
                Ok((
                    match job.status.as_str() {
                        "cancelled" => "cancelled",
                        "succeeded" | "failed" => "already_complete",
                        _ => "cancellation_requested",
                    },
                    json!(job),
                    0,
                ))
            }
            JobAction::Status { job_id } | JobAction::Report { job_id } => {
                let job = get_job(client, *job_id).await?;
                if matches!(action, JobAction::Report { .. }) {
                    Ok((
                        "observed",
                        json!({"complete":job.status == "succeeded","status":job.status,"report":job.report,"last_error":job.last_error}),
                        0,
                    ))
                } else {
                    Ok(("observed", json!(job), 0))
                }
            }
        },
        _ => Err(CliError::configuration("Not a remote command")),
    }
}

async fn finish_submission(
    client: &transport::Transport,
    job: JobStatus,
    options: &WaitOptions,
) -> Result<(&'static str, Value, u8), CliError> {
    if options.wait {
        eprintln!("Job ID: {}", job.id);
        wait(client, job.id, options.timeout).await
    } else {
        Ok(("accepted", json!(job), 0))
    }
}

async fn get_job(client: &transport::Transport, id: Uuid) -> Result<JobStatus, CliError> {
    let job = decode_job(
        client
            .post("/api/import/harvest/status", &json!({"job_id":id}), None)
            .await?,
    )?;
    ensure_job_id(&job, id)?;
    Ok(job)
}

fn decode_job(value: Value) -> Result<JobStatus, CliError> {
    if value.is_null() {
        return Err(CliError::failure(
            "not_found",
            "Import job not found or no longer retained",
        ));
    }
    let job: JobStatus = serde_json::from_value(value)
        .map_err(|_| CliError::failure("protocol", "Invalid job response"))?;
    if !matches!(
        job.kind.as_str(),
        "harvest_api_import" | "harvest_csv_import"
    ) || !matches!(
        job.status.as_str(),
        "queued" | "running" | "succeeded" | "failed" | "cancelled"
    ) {
        return Err(CliError::failure(
            "protocol",
            "Unsupported job kind or state",
        ));
    }
    Ok(job)
}

fn ensure_job_id(job: &JobStatus, requested: Uuid) -> Result<(), CliError> {
    if job.id != requested {
        return Err(CliError::failure(
            "protocol",
            "Server returned a different job",
        ));
    }
    Ok(())
}

async fn wait(
    client: &transport::Transport,
    id: Uuid,
    timeout: Option<u64>,
) -> Result<(&'static str, Value, u8), CliError> {
    let future = async {
        loop {
            let job = get_job(client, id).await?;
            match job.status.as_str() {
                "succeeded" => {
                    let report: ImportReport =
                        serde_json::from_value(job.report.clone().ok_or_else(|| {
                            CliError::failure("protocol", "Completed job has no report")
                        })?)
                        .map_err(|_| {
                            CliError::failure("protocol", "Completed job has an invalid report")
                        })?;
                    return Ok((
                        if report.error_count() > 0 {
                            "partial_success"
                        } else {
                            "succeeded"
                        },
                        json!(job),
                        0,
                    ));
                }
                "failed" => return Ok(("failed", json!(job), 3)),
                "cancelled" => return Ok(("cancelled", json!(job), 4)),
                _ => tokio::time::sleep(Duration::from_secs(1)).await,
            }
        }
    };
    match timeout {
        Some(seconds) => tokio::time::timeout(Duration::from_secs(seconds), future)
            .await
            .map_err(|_| CliError {
                code: 5,
                category: "timeout",
                message: "Wait deadline reached; job was not cancelled".into(),
                http_status: None,
            })?,
        None => future.await,
    }
}

#[cfg(test)]
pub(crate) async fn test_run(args: &[&str], session_file: &std::path::Path) -> (u8, Value) {
    use clap::Parser;
    let cli =
        super::Cli::try_parse_from(std::iter::once("horae").chain(args.iter().copied())).unwrap();
    let result = run_async(
        &cli.command(),
        &RemoteOptions {
            session_file: Some(session_file.into()),
            json: true,
        },
    )
    .await;
    (result.code, serde_json::to_value(&result).unwrap())
}

fn emit(result: &CommandResult, json_output: bool) -> u8 {
    let mut output = std::io::stdout().lock();
    let written = if json_output {
        serde_json::to_writer(&mut output, result)
            .map_err(std::io::Error::other)
            .and_then(|()| writeln!(output))
    } else {
        write_human(&mut output, result)
    };
    if written.and_then(|()| output.flush()).is_err() {
        1
    } else {
        result.code
    }
}

fn write_human(output: &mut impl Write, result: &CommandResult) -> std::io::Result<()> {
    writeln!(output, "{}: {}", result.operation, result.outcome)?;
    if let Some(id) = result.job_id {
        writeln!(output, "Job ID: {id}")?;
    }
    if let Some(id) = result.request_id {
        writeln!(output, "Request ID: {id}")?;
    }
    if let Some(error) = &result.error {
        writeln!(output, "{}", error.message)?;
    }
    if let Some(data) = &result.data {
        serde_json::to_writer_pretty(&mut *output, data).map_err(std::io::Error::other)?;
        writeln!(output)?;
    }
    Ok(())
}
