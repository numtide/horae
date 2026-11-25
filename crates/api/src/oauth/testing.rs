//! What the OAuth router tests share: the router with a session the test
//! can read, form requests, and a seeded person with an app.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use axum::{Extension, Router};
use base64::Engine;
use horae_core::permissions::catalog::BuiltInProfile;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, Session};

use super::OAuthState;
use crate::config::{OAuthConfig, PublicUrl};
use crate::ops::oauth::testing::{App, ISSUER, public_app};
use crate::testing::{SeedIds, seed};

/// A seeded administrator, a public app with every scope, and the server.
pub struct Harness {
    pub ids: SeedIds,
    pub app: App,
    pub session: Session,
    pub router: Router,
    pub pool: PgPool,
}

impl Harness {
    pub async fn new(pool: &PgPool) -> Self {
        let ids = seed(pool, BuiltInProfile::Administrator).await;
        let app = public_app(pool, &ids).await;
        let session = Session::new(None, Arc::new(MemoryStore::default()), None);
        let state = OAuthState::new(
            pool.clone(),
            &PublicUrl::parse(ISSUER).unwrap(),
            OAuthConfig::default(),
        );
        let router = super::router(Some(state)).layer(Extension(session.clone()));
        Self {
            ids,
            app,
            session,
            router,
            pool: pool.clone(),
        }
    }

    pub async fn send(&self, request: Request<Body>) -> Answer {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        Answer {
            status,
            headers,
            body: String::from_utf8(bytes.to_vec()).unwrap(),
        }
    }

    pub async fn get(&self, uri: &str) -> Answer {
        self.send(Request::get(uri).body(Body::empty()).unwrap())
            .await
    }

    /// A form POST with the client in the body.
    pub async fn post(&self, path: &str, form: &[(&str, &str)]) -> Answer {
        self.send(form_request(path, form, None)).await
    }
}

pub struct Answer {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: String,
}

impl Answer {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap()
    }

    pub fn header(&self, name: header::HeaderName) -> Option<&str> {
        self.headers.get(name).map(|value| value.to_str().unwrap())
    }

    /// The `Location` of a redirect, parsed.
    pub fn location(&self) -> url::Url {
        let location = self.header(header::LOCATION).expect("a Location header");
        url::Url::parse(location)
            .or_else(|_| url::Url::parse(&format!("http://horae.test{location}")))
            .unwrap()
    }

    /// The `error` a token endpoint answered.
    pub fn error(&self) -> String {
        self.json()["error"].as_str().unwrap().to_owned()
    }
}

/// A form POST; `basic` sends `(id, secret)` as HTTP Basic.
pub fn form_request(
    path: &str,
    form: &[(&str, &str)],
    basic: Option<(&str, &str)>,
) -> Request<Body> {
    let body: String = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(form)
        .finish();
    let mut request =
        Request::post(path).header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
    if let Some((id, secret)) = basic {
        let encoded = base64::engine::general_purpose::STANDARD.encode(format!("{id}:{secret}"));
        request = request.header(header::AUTHORIZATION, format!("Basic {encoded}"));
    }
    request.body(Body::from(body)).unwrap()
}

/// The query pairs of a URL, as a list.
pub fn query(url: &url::Url) -> Vec<(String, String)> {
    url.query_pairs().into_owned().collect()
}

pub fn param(url: &url::Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}
