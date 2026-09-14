//! Bounded pages from Harvest's data host. Redirects are disabled: only checked
//! same-endpoint pagination links may receive the account and bearer headers.

use std::io::Read;
use std::time::{Duration, Instant};

use anyhow::{Context, bail, ensure};
use chrono::{DateTime, Utc};
use openidconnect::url::Url;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[cfg(test)]
pub(in crate::importers::harvest) mod test_server;

const API_BASE: &str = "https://api.harvestapp.com/v2/";
const PER_PAGE: usize = 100;
const MAX_PAGE_RECORDS: usize = 2_000;
const MAX_PAGE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_LINK_BYTES: usize = 8_192;
const REQUEST_INTERVAL: Duration = Duration::from_millis(160);
const MAX_RETRY_WAIT: Duration = Duration::from_secs(300);

/// The next unconsumed page and Brent cycle state. Persist this with the page's
/// committed work, never just because its HTTP download completed.
#[derive(Clone, Serialize, Deserialize)]
pub(in crate::importers::harvest) struct PageCursor {
    version: u8,
    // Explicit null means EOF; a missing field must not truncate an import.
    #[serde(deserialize_with = "Option::deserialize")]
    next: Option<Url>,
    anchor: Url,
    distance: u64,
    window: u64,
}

impl PageCursor {
    fn advance(&self, next: Option<Url>) -> anyhow::Result<Self> {
        let mut advanced = self.clone();
        if let Some(next) = &next {
            ensure!(next != &self.anchor, "Harvest pagination cycle detected");
            advanced.distance += 1;
            if advanced.distance == advanced.window {
                advanced.anchor = next.clone();
                advanced.window = advanced
                    .window
                    .checked_mul(2)
                    .context("Harvest pagination cursor exhausted")?;
                advanced.distance = 0;
            }
        }
        advanced.next = next;
        Ok(advanced)
    }
}

pub(in crate::importers::harvest) struct ApiHttp {
    agent: ureq::Agent,
    base: Url,
    interval: Duration,
    last_request: Option<Instant>,
}

impl ApiHttp {
    pub(in crate::importers::harvest) fn new() -> anyhow::Result<Self> {
        Ok(Self {
            agent: ureq::AgentBuilder::new()
                .redirects(0)
                .timeout_connect(Duration::from_secs(10))
                // ureq cannot interrupt system DNS resolution; session
                // ownership must survive cancellation even past this deadline.
                .timeout(Duration::from_secs(30))
                .build(),
            base: Url::parse(API_BASE)?,
            interval: REQUEST_INTERVAL,
            last_request: None,
        })
    }

    #[cfg(test)]
    pub(in crate::importers::harvest) fn local(base: Url) -> Self {
        assert_eq!(base.host_str(), Some("127.0.0.1"));
        let mut client = Self::new().unwrap();
        client.base = base;
        client
    }

    pub(in crate::importers::harvest) fn pages<T: DeserializeOwned>(
        &mut self,
        access: &str,
        account: &str,
        collection: &str,
        since: Option<DateTime<Utc>>,
        cancelled: impl Fn() -> bool,
        mut consume: impl FnMut(Vec<T>) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let mut cursor = self.cursor(collection, since)?;
        self.pages_from(
            access,
            account,
            collection,
            &mut cursor,
            cancelled,
            |records, _| consume(records),
        )
    }

    fn endpoint(&self, collection: &str) -> anyhow::Result<Url> {
        ensure!(
            matches!(
                collection,
                "clients" | "projects" | "tasks" | "users" | "time_entries"
            ),
            "unsupported Harvest collection"
        );
        Ok(self.base.join(collection)?)
    }

    pub(in crate::importers::harvest) fn cursor(
        &self,
        collection: &str,
        since: Option<DateTime<Utc>>,
    ) -> anyhow::Result<PageCursor> {
        let mut url = self.endpoint(collection)?;
        url.query_pairs_mut()
            .append_pair("per_page", &PER_PAGE.to_string())
            .append_pair("page", "1");
        if let Some(since) = since {
            url.query_pairs_mut()
                .append_pair("updated_since", &since.to_rfc3339());
        }
        Ok(PageCursor {
            version: 1,
            anchor: url.clone(),
            next: Some(url),
            distance: 0,
            window: 1,
        })
    }

    pub(in crate::importers::harvest) fn pages_from<T: DeserializeOwned>(
        &mut self,
        access: &str,
        account: &str,
        collection: &str,
        cursor: &mut PageCursor,
        cancelled: impl Fn() -> bool,
        mut consume: impl FnMut(Vec<T>, &PageCursor) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let endpoint = self.endpoint(collection)?;
        ensure!(
            cursor.version == 1,
            "unsupported Harvest pagination cursor version"
        );
        ensure!(
            cursor.window.is_power_of_two() && cursor.distance < cursor.window,
            "invalid Harvest pagination cycle state"
        );
        // Stored cursors cross the same trust boundary as provider links. Check
        // against the configured endpoint, not against another stored URL.
        validate_url(&cursor.anchor, &endpoint)?;
        if let Some(next) = &cursor.next {
            validate_url(next, &endpoint)?;
        }
        while let Some(url) = &cursor.next {
            let body = self.get(url, access, account, &cancelled)?;
            let mut page: serde_json::Value =
                serde_json::from_slice(&body).context("invalid Harvest page JSON")?;
            let next = next_url(&page, url)?;
            let items = page
                .get_mut(collection)
                .and_then(serde_json::Value::as_array_mut)
                .context("Harvest page is missing its record array")?;
            ensure!(
                items.len() <= MAX_PAGE_RECORDS,
                "Harvest page exceeds 2000 records"
            );
            let records = std::mem::take(items)
                .into_iter()
                .map(serde_json::from_value)
                .collect::<Result<Vec<T>, _>>()
                .context("invalid Harvest record")?;
            drop(page);
            drop(body);
            ensure!(!cancelled(), "Harvest import cancelled");
            let advanced = cursor.advance(next)?;
            consume(records, &advanced)?;
            *cursor = advanced;
        }
        Ok(())
    }

    fn get(
        &mut self,
        url: &Url,
        access: &str,
        account: &str,
        cancelled: &impl Fn() -> bool,
    ) -> anyhow::Result<Vec<u8>> {
        for attempt in 1..=6 {
            if let Some(last) = self.last_request {
                wait(self.interval.saturating_sub(last.elapsed()), cancelled)?;
            }
            ensure!(!cancelled(), "Harvest import cancelled");
            self.last_request = Some(Instant::now());
            let response = self
                .agent
                .get(url.as_str())
                .set("Authorization", &format!("Bearer {access}"))
                .set("Harvest-Account-Id", account)
                .set("User-Agent", "Horae Importer (support@horae.app)")
                .set("Accept", "application/json")
                .call();
            match response {
                Ok(response) => {
                    ensure!(
                        response.status() == 200,
                        "unexpected Harvest response status"
                    );
                    let mut bytes = Vec::new();
                    response
                        .into_reader()
                        .take(MAX_PAGE_BYTES + 1)
                        .read_to_end(&mut bytes)
                        .context("could not read Harvest page")?;
                    ensure!(
                        bytes.len() as u64 <= MAX_PAGE_BYTES,
                        "Harvest page exceeds 10 MiB"
                    );
                    return Ok(bytes);
                }
                Err(ureq::Error::Status(429, response)) if attempt < 6 => {
                    let seconds = response
                        .header("Retry-After")
                        .unwrap_or("2")
                        .parse::<u64>()
                        .context("invalid Harvest Retry-After")?;
                    let delay = Duration::from_secs(seconds);
                    ensure!(
                        delay <= MAX_RETRY_WAIT,
                        "Harvest throttled for more than five minutes; retry later"
                    );
                    wait(delay, cancelled)?;
                }
                // Do not return provider-controlled URLs/bodies or transport
                // debug output through the user-visible import error.
                Err(ureq::Error::Status(status, _)) => {
                    bail!("Harvest request failed with HTTP {status}")
                }
                Err(ureq::Error::Transport(_)) => bail!("Harvest request failed or timed out"),
            }
        }
        bail!("Harvest request exhausted retries")
    }
}

fn wait(delay: Duration, cancelled: &impl Fn() -> bool) -> anyhow::Result<()> {
    let started = Instant::now();
    loop {
        ensure!(!cancelled(), "Harvest import cancelled");
        let remaining = delay.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            return Ok(());
        }
        std::thread::sleep(remaining.min(Duration::from_millis(100)));
    }
}

fn next_url(page: &serde_json::Value, current: &Url) -> anyhow::Result<Option<Url>> {
    let links = page
        .get("links")
        .context("Harvest page is missing pagination links")?;
    match links
        .get("next")
        .context("Harvest page is missing links.next")?
    {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(link) => {
            ensure!(
                link.len() <= MAX_LINK_BYTES,
                "Harvest pagination link is too long"
            );
            let next = Url::parse(link).context("invalid Harvest pagination link")?;
            validate_url(&next, current)?;
            Ok(Some(next))
        }
        _ => bail!("invalid Harvest links.next"),
    }
}

fn validate_url(next: &Url, endpoint: &Url) -> anyhow::Result<()> {
    ensure!(
        next.as_str().len() <= MAX_LINK_BYTES,
        "Harvest pagination link is too long"
    );
    ensure!(
        next.origin() == endpoint.origin()
            && next.path() == endpoint.path()
            && next.username().is_empty()
            && next.password().is_none()
            && next.fragment().is_none(),
        "Harvest pagination link leaves the collection endpoint"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use test_server::{Response, Server};

    #[test]
    fn persisted_cursor_resumes_with_a_new_client_and_stays_complete_at_eof() {
        use std::cell::Cell;

        let server = Server::start(|url| {
            let resumed = url
                .query_pairs()
                .any(|(key, value)| key == "cursor" && value == "a+b");
            let mut next = url.clone();
            next.query_pairs_mut().clear().append_pair("cursor", "a+b");
            Response::json(json!({
                "time_entries": [if resumed { 2 } else { 1 }],
                "links": {"next": if resumed { None } else { Some(next.as_str()) }}
            }))
        });
        let mut http = ApiHttp::local(server.base.clone());
        let since = "2026-01-01T00:00:00Z".parse().unwrap();
        let mut cursor = http.cursor("time_entries", Some(since)).unwrap();
        let cancelled = Cell::new(false);
        let mut received = vec![];
        let error = http
            .pages_from::<u64>(
                "first-token",
                "account",
                "time_entries",
                &mut cursor,
                || cancelled.get(),
                |records, next| {
                    received.extend(records);
                    assert!(next.next.is_some());
                    cancelled.set(true);
                    Ok(())
                },
            )
            .unwrap_err();
        assert!(error.to_string().contains("cancelled"));
        assert_eq!(received, [1]);
        let saved = serde_json::to_string(&cursor).unwrap();
        assert!(!saved.contains("first-token"));
        let mut cursor: PageCursor = serde_json::from_str(&saved).unwrap();
        let mut replacement = ApiHttp::local(server.base.clone());
        replacement
            .pages_from::<u64>(
                "replacement-token",
                "account",
                "time_entries",
                &mut cursor,
                || false,
                |records, next| {
                    assert!(next.next.is_none());
                    received.extend(records);
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(received, [1, 2]);
        let mut completed = serde_json::from_value(serde_json::to_value(&cursor).unwrap()).unwrap();
        replacement
            .pages_from::<u64>(
                "replacement-token",
                "account",
                "time_entries",
                &mut completed,
                || false,
                |_, _| panic!("completed pagination must not consume again"),
            )
            .unwrap();
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].contains("updated_since="));
        assert!(requests[1].contains("cursor=a%2Bb"));
        assert!(
            requests[1]
                .to_ascii_lowercase()
                .contains("authorization: bearer replacement-token\r\n")
        );
    }

    #[test]
    fn rejected_page_does_not_advance_the_cursor() {
        let server =
            Server::start(|_| Response::json(json!({"clients":[1], "links":{"next":null}})));
        let mut http = ApiHttp::local(server.base.clone());
        let mut cursor = http.cursor("clients", None).unwrap();
        let initial = serde_json::to_value(&cursor).unwrap();
        let error = http
            .pages_from::<u64>(
                "token",
                "account",
                "clients",
                &mut cursor,
                || false,
                |_, _| bail!("batch commit failed"),
            )
            .unwrap_err();
        assert_eq!(error.to_string(), "batch commit failed");
        assert_eq!(serde_json::to_value(&cursor).unwrap(), initial);
    }

    #[test]
    fn missing_next_cursor_is_not_silently_treated_as_completed() {
        let http = ApiHttp::new().unwrap();
        let mut saved = serde_json::to_value(http.cursor("clients", None).unwrap()).unwrap();
        saved.as_object_mut().unwrap().remove("next");
        assert!(serde_json::from_value::<PageCursor>(saved).is_err());
    }

    #[test]
    fn resumed_cursor_rejects_untrusted_destinations_and_invalid_state_before_http() {
        let server = Server::start(|_| panic!("invalid cursors must not send credentials"));
        let mut http = ApiHttp::local(server.base.clone());
        let valid = serde_json::to_value(http.cursor("clients", None).unwrap()).unwrap();
        for (field, value) in [
            ("version", json!(2)),
            ("window", json!(0)),
            ("window", json!(3)),
            ("distance", json!(1)),
            (
                "next",
                json!(server.base.join("users?page=2").unwrap().as_str()),
            ),
            ("next", json!("https://example.invalid/v2/clients?page=2")),
            ("anchor", json!("https://example.invalid/v2/clients?page=1")),
            (
                "next",
                json!(format!(
                    "{}clients?cursor={}",
                    server.base,
                    "x".repeat(MAX_LINK_BYTES)
                )),
            ),
            (
                "next",
                json!(format!("{}clients?page=2#fragment", server.base)),
            ),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            let mut cursor = serde_json::from_value(invalid).unwrap();
            assert!(
                http.pages_from::<u64>(
                    "token",
                    "account",
                    "clients",
                    &mut cursor,
                    || false,
                    |_, _| panic!("invalid cursor consumed a page"),
                )
                .is_err(),
                "accepted invalid {field}"
            );
        }
        assert!(server.requests.lock().unwrap().is_empty());
    }

    #[test]
    fn pagination_cycle_detection_survives_restarting_after_every_page() {
        use std::cell::Cell;

        let server = Server::start(|url| {
            let second = url.query_pairs().any(|(_, value)| value == "two");
            let mut next = url.clone();
            next.query_pairs_mut()
                .clear()
                .append_pair("cursor", if second { "one" } else { "two" });
            Response::json(json!({"clients":[], "links":{"next":next.as_str()}}))
        });
        let mut saved = serde_json::to_value(
            ApiHttp::local(server.base.clone())
                .cursor("clients", None)
                .unwrap(),
        )
        .unwrap();
        for _ in 0..8 {
            let mut cursor = serde_json::from_value(saved).unwrap();
            let cancelled = Cell::new(false);
            let error = ApiHttp::local(server.base.clone())
                .pages_from::<u64>(
                    "token",
                    "account",
                    "clients",
                    &mut cursor,
                    || cancelled.get(),
                    |_, _| {
                        cancelled.set(true);
                        Ok(())
                    },
                )
                .unwrap_err();
            if error.to_string().contains("cycle") {
                assert!(server.requests.lock().unwrap().len() < 8);
                return;
            }
            assert!(error.to_string().contains("cancelled"));
            saved = serde_json::to_value(&cursor).unwrap();
        }
        panic!("restarts reset pagination cycle detection");
    }

    #[test]
    fn http_pages_follow_cursor_and_preserve_headers_and_encoded_filter() {
        let server = Server::start(|url| {
            let cursor = url
                .query_pairs()
                .any(|(key, value)| key == "cursor" && value == "a+b");
            let mut next = url.clone();
            next.query_pairs_mut().clear().append_pair("cursor", "a+b");
            Response::json(
                json!({"time_entries":[if cursor {2} else {1}], "next_page":null, "links":{"next":if cursor {None} else {Some(next.as_str())}}}),
            )
        });
        let mut client = ApiHttp::local(server.base.clone());
        let since = "2026-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let mut received = vec![];
        client
            .pages::<u64>(
                "test-token",
                "test-account",
                "time_entries",
                Some(since),
                || false,
                |page| {
                    received.push(page);
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(received, vec![vec![1], vec![2]]);
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        for request in requests.iter() {
            let lower = request.to_ascii_lowercase();
            assert!(lower.contains("authorization: bearer test-token\r\n"));
            assert!(lower.contains("harvest-account-id: test-account\r\n"));
            assert!(lower.contains("user-agent: horae importer"));
        }
        let initial = server
            .base
            .join(requests[0].split_whitespace().nth(1).unwrap())
            .unwrap();
        assert!(
            initial
                .query_pairs()
                .any(|(key, value)| key == "updated_since" && value == since.to_rfc3339())
        );
        assert!(requests[0].contains("%2B00%3A00"));
    }

    #[test]
    fn http_retries_429_but_never_follows_redirects_or_shortens_long_backoff() {
        let mut count = 0;
        let server = Server::start(move |_| {
            count += 1;
            if count == 1 {
                Response {
                    status: 429,
                    headers: vec![("Retry-After", "0".into())],
                    body: vec![],
                }
            } else {
                Response::json(json!({"clients":[1], "links":{"next":null}}))
            }
        });
        ApiHttp::local(server.base.clone())
            .pages::<u64>("token", "account", "clients", None, || false, |_| Ok(()))
            .unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 2);
        for (status, header, value) in [
            (302, "Location", "https://example.invalid/"),
            (429, "Retry-After", "301"),
        ] {
            let server = Server::start(move |_| Response {
                status,
                headers: vec![(header, value.into())],
                body: vec![],
            });
            assert!(
                ApiHttp::local(server.base.clone())
                    .pages::<u64>("token", "account", "clients", None, || false, |_| Ok(()))
                    .is_err()
            );
            assert_eq!(server.requests.lock().unwrap().len(), 1);
        }
    }

    #[test]
    fn http_rejects_oversized_malformed_and_cyclic_pages() {
        for body in [
            serde_json::to_vec(
                &json!({"clients":vec![1; MAX_PAGE_RECORDS + 1], "links":{"next":null}}),
            )
            .unwrap(),
            vec![b' '; MAX_PAGE_BYTES as usize + 1],
            b"{\"clients\":[],\"links\":{\"next\":null},\"bad\":\"\xff\"}".to_vec(),
            serde_json::to_vec(&json!({"links":{"next":null}})).unwrap(),
        ] {
            let server = Server::start(move |_| Response {
                status: 200,
                headers: vec![],
                body: body.clone(),
            });
            assert!(
                ApiHttp::local(server.base.clone())
                    .pages::<u64>("token", "account", "clients", None, || false, |_| Ok(()))
                    .is_err()
            );
        }
        let server = Server::start(|url| {
            let mut next = url.clone();
            let cursor = url.query_pairs().any(|(_, value)| value == "two");
            next.query_pairs_mut()
                .clear()
                .append_pair("cursor", if cursor { "one" } else { "two" });
            Response::json(json!({"clients":[], "links":{"next":next.as_str()}}))
        });
        let error = ApiHttp::local(server.base.clone())
            .pages::<u64>("token", "account", "clients", None, || false, |_| Ok(()))
            .unwrap_err();
        assert!(error.to_string().contains("cycle"));
        assert!(server.requests.lock().unwrap().len() < 8);
    }

    #[test]
    fn slow_consumer_backpressures_http_and_cancellation_stops_further_pages() {
        let (requested, requests) = std::sync::mpsc::channel();
        let server = Server::start(move |url| {
            let page = url
                .query_pairs()
                .find(|(key, _)| key == "page")
                .unwrap()
                .1
                .parse::<u64>()
                .unwrap();
            requested.send(page).unwrap();
            let mut next = url.clone();
            next.query_pairs_mut()
                .clear()
                .append_pair("page", &(page + 1).to_string());
            Response::json(json!({"clients":[page], "links":{"next":next.as_str()}}))
        });
        let mut http = ApiHttp::local(server.base.clone());
        let (send, receive) = tokio::sync::mpsc::channel(1);
        let worker = std::thread::spawn(move || {
            http.pages::<u64>(
                "token",
                "account",
                "clients",
                None,
                || send.is_closed(),
                |page| send.blocking_send(page).map_err(Into::into),
            )
        });
        assert_eq!(requests.recv_timeout(Duration::from_secs(5)).unwrap(), 1);
        assert_eq!(requests.recv_timeout(Duration::from_secs(5)).unwrap(), 2);
        let next = requests.recv_timeout(REQUEST_INTERVAL * 2);
        drop(receive);
        let result = worker.join().unwrap();
        assert!(matches!(
            next,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ));
        assert!(result.is_err());
        assert_eq!(server.requests.lock().unwrap().len(), 2);
    }

    #[test]
    fn cursor_links_are_authoritative_even_with_null_or_stale_page_numbers() {
        let current = Url::parse("https://api.harvestapp.com/v2/time_entries?page=1").unwrap();
        let link = "https://api.harvestapp.com/v2/time_entries?cursor=a%2Bb&updated_since=2026-01-01T00%3A00%3A00Z";
        for numeric in [json!(null), json!(1), json!(2)] {
            assert_eq!(
                next_url(
                    &json!({"next_page": numeric, "links":{"next":link}}),
                    &current
                )
                .unwrap()
                .unwrap()
                .as_str(),
                link
            );
        }
        assert!(
            next_url(&json!({"links":{"next":null}}), &current)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn rejects_pagination_outside_the_authenticated_endpoint() {
        let current = Url::parse("https://api.harvestapp.com/v2/time_entries?page=1").unwrap();
        for link in [
            "http://api.harvestapp.com/v2/time_entries?page=2",
            "https://api.harvestapp.com.evil.test/v2/time_entries?page=2",
            "https://api.harvestapp.com:444/v2/time_entries?page=2",
            "https://api.harvestapp.com/v2/users?page=2",
            "https://api.harvestapp.com/v2/../time_entries?page=2",
            "https://user:password@api.harvestapp.com/v2/time_entries?page=2",
            "https://api.harvestapp.com/v2/time_entries?page=2#fragment",
            "/v2/time_entries?page=2",
        ] {
            assert!(
                next_url(&json!({"links":{"next":link}}), &current).is_err(),
                "{link}"
            );
        }
        for page in [
            json!({}),
            json!({"next_page":null}),
            json!({"links":{}}),
            json!({"links":{"next":2}}),
        ] {
            assert!(next_url(&page, &current).is_err());
        }
    }

    #[test]
    fn cancelled_backoff_does_not_wait_or_retry() {
        assert!(wait(Duration::from_secs(300), &|| true).is_err());
    }
}
