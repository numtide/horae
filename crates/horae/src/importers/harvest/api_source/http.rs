//! Bounded pages from Harvest's data host. Redirects are disabled: only checked
//! same-endpoint pagination links may receive the account and bearer headers.

use std::io::Read;
use std::time::{Duration, Instant};

use anyhow::{Context, bail, ensure};
use chrono::{DateTime, Utc};
use openidconnect::url::Url;
use serde::de::DeserializeOwned;

#[cfg(test)]
pub(in crate::importers::harvest) mod test_server;

const API_BASE: &str = "https://api.harvestapp.com/v2/";
const PER_PAGE: usize = 100;
const MAX_PAGE_RECORDS: usize = 2_000;
const MAX_PAGE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_LINK_BYTES: usize = 8_192;
const REQUEST_INTERVAL: Duration = Duration::from_millis(160);
const MAX_RETRY_WAIT: Duration = Duration::from_secs(300);

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
        ensure!(
            matches!(
                collection,
                "clients" | "projects" | "tasks" | "users" | "time_entries"
            ),
            "unsupported Harvest collection"
        );
        let mut url = self.base.join(collection)?;
        url.query_pairs_mut()
            .append_pair("per_page", &PER_PAGE.to_string())
            .append_pair("page", "1");
        if let Some(since) = since {
            url.query_pairs_mut()
                .append_pair("updated_since", &since.to_rfc3339());
        }
        // Brent's cycle detector retains one checkpoint, not every page URL.
        let mut checkpoint = url.clone();
        let mut distance = 0usize;
        let mut window = 1usize;
        loop {
            let body = self.get(&url, access, account, &cancelled)?;
            let mut page: serde_json::Value =
                serde_json::from_slice(&body).context("invalid Harvest page JSON")?;
            let next = next_url(&page, &url)?;
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
            consume(records)?;
            let Some(next) = next else { return Ok(()) };
            distance += 1;
            ensure!(next != checkpoint, "Harvest pagination cycle detected");
            if distance == window {
                checkpoint = next.clone();
                window = window.saturating_mul(2);
                distance = 0;
            }
            url = next;
        }
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
            ensure!(
                next.origin() == current.origin()
                    && next.path() == current.path()
                    && next.username().is_empty()
                    && next.password().is_none()
                    && next.fragment().is_none(),
                "Harvest pagination link leaves the collection endpoint"
            );
            Ok(Some(next))
        }
        _ => bail!("invalid Harvest links.next"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use test_server::{Response, Server};

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
