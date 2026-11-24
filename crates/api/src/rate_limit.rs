//! The per-token request limit (research R-7): a fixed window per token held
//! in memory, which is enough while Horae runs as one process. Every API that
//! authenticates with tokens shares one [`RateLimiter`], so a token's requests
//! count together wherever they go.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use uuid::Uuid;

use crate::config::RateLimit;
use crate::ops::{Caller, Channel};
use crate::problem::Problem;

/// Forget finished windows once this many tokens are tracked.
const PRUNE_AT: usize = 1024;

/// Counts requests per key: a token id for the APIs, a client id for the
/// OAuth token endpoint.
#[derive(Debug)]
pub struct RateLimiter<K = Uuid> {
    limit: RateLimit,
    windows: Mutex<HashMap<K, Window>>,
}

#[derive(Debug, Clone, Copy)]
struct Window {
    started: Instant,
    requests: u32,
}

impl<K: Eq + std::hash::Hash> RateLimiter<K> {
    pub fn new(limit: RateLimit) -> Self {
        Self {
            limit,
            windows: Mutex::default(),
        }
    }

    /// Counts one request of `token`. Over the limit, the answer is how long
    /// until its window ends.
    pub fn check(&self, token: K, now: Instant) -> Result<(), Duration> {
        // A panic elsewhere while counting leaves the counts usable.
        let mut windows = self
            .windows
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let fresh = Window {
            started: now,
            requests: 0,
        };
        if windows.len() >= PRUNE_AT && !windows.contains_key(&token) {
            windows.retain(|_, window| now.duration_since(window.started) < self.limit.window);
        }
        let window = windows.entry(token).or_insert(fresh);
        if now.duration_since(window.started) >= self.limit.window {
            *window = fresh;
        }
        if window.requests >= self.limit.requests {
            return Err(self.limit.window - now.duration_since(window.started));
        }
        window.requests += 1;
        Ok(())
    }
}

/// Middleware for routes whose [`Caller`] an earlier layer put in the request
/// extensions: a token over its limit is answered 429 with `Retry-After`.
/// Requests without a token caller pass.
pub async fn limit(
    State(limiter): State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Response {
    let token = match request.extensions().get::<Caller>() {
        Some(Caller {
            channel: Channel::Token { token_id },
            ..
        }) => Some(*token_id),
        _ => None,
    };
    if let Some(token) = token
        && let Err(wait) = limiter.check(token, Instant::now())
    {
        return too_many_requests(wait);
    }
    next.run(request).await
}

/// The `Retry-After` for a refusal that must wait `wait`: whole seconds,
/// rounded up, and never 0, so a client that obeys does not come back inside
/// the window.
pub fn retry_after_seconds(wait: Duration) -> u64 {
    let seconds = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
    seconds.max(1)
}

fn too_many_requests(wait: Duration) -> Response {
    let seconds = retry_after_seconds(wait);
    let problem = Problem::new(
        StatusCode::TOO_MANY_REQUESTS,
        "rate_limited",
        Some(format!(
            "Too many requests for this token; retry in {seconds} s"
        )),
    );
    let mut response = problem.into_response();
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from(seconds));
    response
}

#[cfg(test)]
mod tests;
