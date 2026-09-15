use std::{fs::File, io::Read, path::Path, time::Duration};

use dioxus::prelude::dioxus_fullstack::reqwest::{self, Client, Response, Url};
use serde::Deserialize;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::CliError;

pub(super) const MAX_UPLOAD: u64 = 50 * 1024 * 1024;
const MAX_JSON: usize = 4 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionFile {
    server_origin: String,
    session_id: String,
}

pub(super) struct Transport {
    client: Client,
    origin: Url,
    cookie: reqwest::header::HeaderValue,
}

impl Transport {
    pub(super) fn load(path: &Path) -> Result<Self, CliError> {
        let invalid = || CliError::configuration("Cannot read a private, regular session file");
        let before = std::fs::symlink_metadata(path).map_err(|_| invalid())?;
        if !before.is_file() || before.len() > 16 * 1024 {
            return Err(invalid());
        }
        let mut file = File::open(path).map_err(|_| invalid())?;
        let metadata = file.metadata().map_err(|_| invalid())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.mode() & 0o077 != 0
                || metadata.dev() != before.dev()
                || metadata.ino() != before.ino()
            {
                return Err(invalid());
            }
        }
        if !metadata.is_file() || metadata.len() > 16 * 1024 {
            return Err(invalid());
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(16 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| invalid())?;
        if bytes.len() > 16 * 1024 {
            return Err(invalid());
        }
        let session: SessionFile = serde_json::from_slice(&bytes)
            .map_err(|_| CliError::configuration("Invalid session file schema"))?;
        let origin = validate_origin(&session.server_origin)?;
        if session.session_id.is_empty()
            || session.session_id.len() > 256
            || !session
                .session_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_=".contains(&b))
        {
            return Err(CliError::configuration("Invalid session cookie value"));
        }
        let mut cookie =
            reqwest::header::HeaderValue::from_str(&format!("id={}", session.session_id))
                .map_err(|_| CliError::configuration("Invalid session cookie value"))?;
        cookie.set_sensitive(true);
        let client = Client::builder()
            .use_rustls_tls()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| CliError::configuration("Cannot initialize the HTTP client"))?;
        Ok(Self {
            client,
            origin,
            cookie,
        })
    }

    fn request(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> Result<reqwest::RequestBuilder, CliError> {
        let url = self
            .origin
            .join(path)
            .map_err(|_| CliError::failure("protocol", "Invalid request path"))?;
        if url.origin() != self.origin.origin() {
            return Err(CliError::failure(
                "protocol",
                "Request destination does not match session origin",
            ));
        }
        Ok(self
            .client
            .request(method, url)
            .header(reqwest::header::COOKIE, self.cookie.clone())
            .header(reqwest::header::ACCEPT, "application/json")
            .timeout(Duration::from_secs(30)))
    }

    pub(super) async fn post(
        &self,
        path: &str,
        body: &Value,
        key: Option<uuid::Uuid>,
    ) -> Result<Value, CliError> {
        let mut request = self.request(reqwest::Method::POST, path)?.json(body);
        if let Some(key) = key {
            request = request.header("X-Horae-Idempotency-Key", key.to_string());
        }
        let response = request
            .send()
            .await
            .map_err(|_| network_error(key.is_some()))?;
        read_json(response, key.is_some()).await
    }

    pub(super) async fn csv(
        &self,
        path: &str,
        file: &Path,
        key: uuid::Uuid,
    ) -> Result<Value, CliError> {
        // Opening a FIFO can block before there is a handle to inspect.
        let metadata = tokio::fs::metadata(file)
            .await
            .map_err(|_| CliError::configuration("Cannot inspect CSV file"))?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_UPLOAD {
            return Err(CliError::configuration(
                "CSV must be a nonempty regular file of at most 50 MiB",
            ));
        }
        let file = tokio::fs::File::open(file)
            .await
            .map_err(|_| CliError::configuration("Cannot open CSV file"))?;
        let metadata = file
            .metadata()
            .await
            .map_err(|_| CliError::configuration("Cannot inspect CSV file"))?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_UPLOAD {
            return Err(CliError::configuration(
                "CSV must be a nonempty regular file of at most 50 MiB",
            ));
        }
        let stream =
            futures_util::stream::try_unfold((file, 0u64), |(mut file, count)| async move {
                let mut bytes = vec![0; 64 * 1024];
                let read = file.read(&mut bytes).await?;
                if count + read as u64 > MAX_UPLOAD {
                    return Err(std::io::Error::other("CSV exceeds upload limit"));
                }
                bytes.truncate(read);
                Ok::<_, std::io::Error>((read != 0).then_some((bytes, (file, count + read as u64))))
            });
        let response = self
            .request(reqwest::Method::POST, path)?
            .header("X-Horae-Import", "csv")
            .header("X-Horae-Idempotency-Key", key.to_string())
            .header(reqwest::header::CONTENT_TYPE, "text/csv")
            .body(reqwest::Body::wrap_stream(stream))
            .timeout(Duration::from_secs(300))
            .send()
            .await
            .map_err(|_| network_error(true))?;
        read_json(response, true).await
    }

    pub(super) async fn download(
        &self,
        path: &str,
        output: &Path,
        force: bool,
    ) -> Result<u64, CliError> {
        if !force
            && output
                .try_exists()
                .map_err(|_| CliError::failure("output", "Cannot inspect output destination"))?
        {
            return Err(CliError::failure(
                "output",
                "Output already exists; use --force to replace it",
            ));
        }
        let parent = output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let temporary = TemporaryFile(parent.join(format!(".horae-{}.part", uuid::Uuid::now_v7())));
        let mut options = tokio::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options
            .open(&temporary.0)
            .await
            .map_err(|_| CliError::failure("output", "Cannot create temporary output file"))?;
        let mut response = self
            .request(reqwest::Method::GET, path)?
            .timeout(Duration::from_secs(300))
            .send()
            .await
            .map_err(|_| network_error(false))?;
        check_status(&response, false)?;
        if response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_none_or(|value| !value.starts_with("application/x-ndjson"))
        {
            return Err(CliError::failure(
                "protocol",
                "Server did not return an error archive",
            ));
        }
        let mut count = 0u64;
        while let Some(bytes) = response
            .chunk()
            .await
            .map_err(|_| CliError::failure("download", "Error archive download was interrupted"))?
        {
            file.write_all(&bytes)
                .await
                .map_err(|_| CliError::failure("output", "Cannot write error archive"))?;
            count = count
                .checked_add(bytes.len() as u64)
                .ok_or_else(|| CliError::failure("download", "Archive size overflow"))?;
        }
        file.sync_all()
            .await
            .map_err(|_| CliError::failure("output", "Cannot synchronize error archive"))?;
        drop(file);
        if force {
            tokio::fs::rename(&temporary.0, output).await
        } else {
            tokio::fs::hard_link(&temporary.0, output).await
        }
        .map_err(|_| {
            CliError::failure(
                "output",
                "Cannot publish archive; destination left unchanged",
            )
        })?;
        Ok(count)
    }
}

fn validate_origin(value: &str) -> Result<Url, CliError> {
    let invalid = || {
        CliError::configuration(
            "Session origin must be HTTPS (or loopback HTTP), without credentials, query, fragment or path prefix",
        )
    };
    let url = Url::parse(value).map_err(|_| invalid())?;
    let host = url.host_str().ok_or_else(invalid)?;
    let loopback = host == "localhost"
        || host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
    if !(url.scheme() == "https" || url.scheme() == "http" && loopback)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        return Err(invalid());
    }
    Ok(url)
}

fn network_error(submitting: bool) -> CliError {
    if submitting {
        CliError { code: 6, category: "indeterminate_submission", message: "Submission acknowledgement unavailable; resubmit identical input with the same request ID".into(), http_status: None }
    } else {
        CliError::failure("transport", "Server unavailable or request interrupted")
    }
}

fn check_status(response: &Response, submitting: bool) -> Result<(), CliError> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    if submitting && status.is_server_error() {
        return Err(network_error(true));
    }
    let message = match status.as_u16() {
        401 => "Session expired or unavailable; sign in again",
        403 => "Active administrator access is required",
        404 => "Job, connection or compatible server endpoint not found",
        409 => "Conflicting request or job state",
        413 => "CSV exceeds the server upload limit",
        _ if status.is_redirection() => "Server redirected the request; redirects are not followed",
        _ => "Server rejected the request",
    };
    Err(CliError {
        code: 1,
        category: "http",
        message: message.into(),
        http_status: Some(status.as_u16()),
    })
}

async fn read_json(mut response: Response, submitting: bool) -> Result<Value, CliError> {
    check_status(&response, submitting)?;
    let mut body = Vec::new();
    while let Some(bytes) = response
        .chunk()
        .await
        .map_err(|_| network_error(submitting))?
    {
        if body.len().saturating_add(bytes.len()) > MAX_JSON {
            return Err(if submitting {
                network_error(true)
            } else {
                CliError::failure("protocol", "Server response exceeds the JSON limit")
            });
        }
        body.extend_from_slice(&bytes);
    }
    serde_json::from_slice(&body).map_err(|_| {
        if submitting {
            network_error(true)
        } else {
            CliError::failure("protocol", "Server returned invalid JSON")
        }
    })
}

struct TemporaryFile(std::path::PathBuf);
impl Drop for TemporaryFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
