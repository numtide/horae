//! Native browser file upload without serializing the file into a JSON vector.

use dioxus::html::FileData;
use dioxus::prelude::*;
#[cfg(feature = "server")]
use dioxus_fullstack::RequestError;
#[cfg(feature = "server")]
use dioxus_fullstack::body::Body;
#[cfg(feature = "server")]
use dioxus_fullstack::extract::{FromRequest, Request};
use dioxus_fullstack::{ClientRequest, ClientResult, FileStream, IntoRequest};

pub enum CsvUpload {
    File(FileData),
    #[cfg(feature = "server")]
    Body(Body),
}

impl From<FileData> for CsvUpload {
    fn from(file: FileData) -> Self {
        Self::File(file)
    }
}

impl IntoRequest for CsvUpload {
    async fn into_request(self, request: ClientRequest) -> ClientResult {
        let file = match self {
            Self::File(file) => file,
            #[cfg(feature = "server")]
            Self::Body(_) => {
                return Err(RequestError::Request(
                    "CSV upload has no selected file".into(),
                ));
            }
        };
        FileStream::from(file)
            .into_request(request.header("X-Horae-Import", "csv")?)
            .await
    }
}

#[cfg(feature = "server")]
impl<S: Sync> FromRequest<S> for CsvUpload {
    type Rejection = ServerFnError;

    async fn from_request(request: Request, _: &S) -> Result<Self, Self::Rejection> {
        // Require a non-simple request: cross-site HTML forms cannot supply
        // this header. Do not enable cross-origin credentialed CORS here.
        if request
            .headers()
            .get("X-Horae-Import")
            .is_none_or(|value| value != "csv")
        {
            return Err(ServerFnError::ServerError {
                code: dioxus_fullstack::StatusCode::FORBIDDEN.as_u16(),
                message: "Missing CSV upload header".into(),
                details: None,
            });
        }
        // Extract only the body, without logging request headers or cookies.
        Ok(Self::Body(request.into_body()))
    }
}

#[cfg(feature = "server")]
impl CsvUpload {
    pub(super) fn into_body(self) -> Result<Body, ServerFnError> {
        match self {
            Self::Body(body) => Ok(body),
            Self::File(_) => Err(ServerFnError::new("CSV upload has no request body")),
        }
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn csv_upload_requires_its_header_without_reading_the_body() {
        for header in [None, Some("wrong"), Some("CSV")] {
            let stream = futures_util::stream::poll_fn(
                |_| -> std::task::Poll<
                    Option<Result<dioxus_fullstack::body::Bytes, std::io::Error>>,
                > {
                    panic!("unauthorized upload body was read");
                },
            );
            let mut request = Request::builder().method("POST");
            if let Some(header) = header {
                request = request.header("X-Horae-Import", header);
            }
            let result =
                CsvUpload::from_request(request.body(Body::from_stream(stream)).unwrap(), &())
                    .await;
            assert!(
                matches!(result, Err(ServerFnError::ServerError { code, .. })
                if code == dioxus_fullstack::StatusCode::FORBIDDEN.as_u16())
            );
        }
    }

    #[tokio::test]
    async fn csv_upload_extractor_preserves_raw_bytes() {
        let bytes = b"Date,Notes\n2026-01-15,\"quoted, note\"\n";
        let request = Request::builder()
            .method("POST")
            .uri("/api/import/harvest/csv/DryRun")
            .header("X-Horae-Import", "csv")
            .body(Body::from(bytes.as_slice()))
            .unwrap();
        let upload = CsvUpload::from_request(request, &()).await.unwrap();
        let actual = axum::body::to_bytes(upload.into_body().unwrap(), 1024)
            .await
            .unwrap();
        assert_eq!(&actual[..], bytes);
    }
}
