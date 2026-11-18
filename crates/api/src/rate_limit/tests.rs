use super::*;
use axum::Router;
use axum::body::Body;
use axum::routing::get;
use tower::ServiceExt;

fn limiter(requests: u32, seconds: u64) -> RateLimiter {
    RateLimiter::new(RateLimit {
        requests,
        window: Duration::from_secs(seconds),
    })
}

#[test]
fn a_token_gets_its_requests_per_window_then_waits_for_the_window_to_end() {
    let limiter = limiter(3, 15);
    let token = Uuid::now_v7();
    let start = Instant::now();
    for second in 0..3 {
        assert_eq!(
            limiter.check(token, start + Duration::from_secs(second)),
            Ok(())
        );
    }
    assert_eq!(
        limiter.check(token, start + Duration::from_secs(5)),
        Err(Duration::from_secs(10))
    );
    assert_eq!(
        limiter.check(token, start + Duration::from_millis(14_500)),
        Err(Duration::from_millis(500))
    );
}

#[test]
fn the_window_resets_when_it_ends() {
    let limiter = limiter(1, 15);
    let token = Uuid::now_v7();
    let start = Instant::now();
    assert_eq!(limiter.check(token, start), Ok(()));
    assert!(
        limiter
            .check(token, start + Duration::from_secs(14))
            .is_err()
    );
    let next = start + Duration::from_secs(15);
    assert_eq!(limiter.check(token, next), Ok(()));
    assert!(limiter.check(token, next).is_err());
}

#[test]
fn each_token_has_its_own_window() {
    let limiter = limiter(1, 15);
    let start = Instant::now();
    let (first, second) = (Uuid::now_v7(), Uuid::now_v7());
    assert_eq!(limiter.check(first, start), Ok(()));
    assert!(limiter.check(first, start).is_err());
    assert_eq!(limiter.check(second, start), Ok(()));
}

#[test]
fn finished_windows_are_forgotten_without_losing_running_ones() {
    let limiter = limiter(1, 15);
    let start = Instant::now();
    let busy = Uuid::now_v7();
    assert_eq!(limiter.check(busy, start + Duration::from_secs(20)), Ok(()));
    for _ in 1..PRUNE_AT {
        limiter.check(Uuid::now_v7(), start).unwrap();
    }
    let later = start + Duration::from_secs(30);
    limiter.check(Uuid::now_v7(), later).unwrap();
    assert_eq!(limiter.windows.lock().unwrap().len(), 2);
    assert!(limiter.check(busy, later).is_err(), "busy kept its count");
}

async fn call(router: &Router, caller: Option<Caller>) -> Response {
    let mut request = Request::builder().uri("/").body(Body::empty()).unwrap();
    if let Some(caller) = caller {
        request.extensions_mut().insert(caller);
    }
    router.clone().oneshot(request).await.unwrap()
}

#[tokio::test]
async fn over_the_limit_the_answer_is_429_with_retry_after() {
    let limiter = Arc::new(limiter(1, 15));
    let router = Router::new()
        .route("/", get(|| async { "ok" }))
        .route_layer(axum::middleware::from_fn_with_state(limiter, limit));
    let token = Caller::token(Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());

    assert_eq!(call(&router, Some(token)).await.status(), StatusCode::OK);
    let refused = call(&router, Some(token)).await;
    assert_eq!(refused.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry_after: u64 = refused.headers()[header::RETRY_AFTER]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((1..=15).contains(&retry_after), "{retry_after}");
    assert_eq!(
        refused.headers()[header::CONTENT_TYPE],
        crate::problem::CONTENT_TYPE
    );
    let body = axum::body::to_bytes(refused.into_body(), usize::MAX)
        .await
        .unwrap();
    let problem: Problem = serde_json::from_slice(&body).unwrap();
    assert_eq!(problem.code, "rate_limited");

    let session = Caller::session(Uuid::now_v7(), Uuid::now_v7());
    for _ in 0..3 {
        assert_eq!(call(&router, Some(session)).await.status(), StatusCode::OK);
    }
    assert_eq!(call(&router, None).await.status(), StatusCode::OK);
}

#[test]
fn retry_after_rounds_up_to_whole_seconds() {
    for (wait, seconds) in [
        (Duration::from_millis(1), "1"),
        (Duration::from_millis(1500), "2"),
        (Duration::from_secs(3), "3"),
        (Duration::ZERO, "1"),
    ] {
        assert_eq!(
            too_many_requests(wait).headers()[header::RETRY_AFTER],
            seconds
        );
    }
}
