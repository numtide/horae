//! Loopback-only HTTP fixture that generates one response at a time.

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread::JoinHandle;
use std::time::Duration;

use openidconnect::url::Url;

pub(in crate::importers::harvest) struct Response {
    pub status: u16,
    pub headers: Vec<(&'static str, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn json(body: serde_json::Value) -> Self {
        Self {
            status: 200,
            headers: vec![],
            body: serde_json::to_vec(&body).unwrap(),
        }
    }
}

pub(in crate::importers::harvest) struct Server {
    pub base: Url,
    pub requests: Arc<Mutex<Vec<String>>>,
    pub count: Arc<AtomicUsize>,
    address: SocketAddr,
    stopped: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Server {
    pub fn start(mut response: impl FnMut(&Url) -> Response + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let base = Url::parse(&format!("http://{address}/v2/")).unwrap();
        let stopped = Arc::new(AtomicBool::new(false));
        let worker_stopped = stopped.clone();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let worker_requests = requests.clone();
        let count = Arc::new(AtomicUsize::new(0));
        let worker_count = count.clone();
        let origin = base.clone();
        let worker = std::thread::spawn(move || {
            for connection in listener.incoming() {
                if worker_stopped.load(Ordering::Acquire) {
                    break;
                }
                let mut connection = connection.unwrap();
                connection
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                connection
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(&mut connection);
                let mut request = String::new();
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        break;
                    }
                    request.push_str(&line);
                    if line == "\r\n" {
                        break;
                    }
                    assert!(request.len() < 16 * 1024);
                }
                let Some(target) = request.split_whitespace().nth(1) else {
                    continue;
                };
                let url = origin.join(target).unwrap();
                worker_count.fetch_add(1, Ordering::Release);
                // Keep scale-fixture memory bounded too; protocol tests can
                // inspect the first requests' complete headers.
                let mut recorded = worker_requests.lock().unwrap();
                if recorded.len() < 16 {
                    recorded.push(request);
                }
                drop(recorded);
                let response = response(&url);
                let mut headers = format!(
                    "HTTP/1.1 {} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
                    response.status,
                    response.body.len()
                );
                for (name, value) in response.headers {
                    headers.push_str(&format!("{name}: {value}\r\n"));
                }
                headers.push_str("\r\n");
                // A size/timeout rejection may close the socket before the
                // complete fixture response has been sent.
                if connection.write_all(headers.as_bytes()).is_ok() {
                    let _ = connection.write_all(&response.body);
                }
            }
        });
        Self {
            base,
            requests,
            count,
            address,
            stopped,
            worker: Some(worker),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        let _ = TcpStream::connect_timeout(&self.address, Duration::from_secs(1));
        if let Some(worker) = self.worker.take() {
            let result = worker.join();
            if !std::thread::panicking() {
                result.expect("HTTP fixture worker panicked");
            }
        }
    }
}
