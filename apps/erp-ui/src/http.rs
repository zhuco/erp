//! Bounded, cancellable host HTTP. Secrets are owned only by the request/session,
//! never persisted, logged or included in transport diagnostics.
use futures::{
    channel::oneshot,
    future::{AbortHandle, Abortable},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

mod download;
pub use download::DownloadTask;

pub const MAX_BODY: usize = 8 * 1024 * 1024;
pub const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HttpError {
    InvalidAddress,
    Status(u16),
    TooLarge,
    Transport,
    Cancelled,
}
impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAddress => f.write_str("Invalid server address"),
            Self::Status(code) => write!(f, "HTTP {code}"),
            Self::TooLarge => f.write_str("Response exceeds 8 MiB"),
            Self::Transport => f.write_str("Connection failed or timed out"),
            Self::Cancelled => f.write_str("Request cancelled"),
        }
    }
}
pub struct HttpRequest {
    pub path: String,
    pub query: Vec<(String, String)>,
    pub json: Option<Vec<u8>>,
    pub bearer: Option<String>,
    /// Shared command identity. Sent as `Idempotency-Key` and must match a body `request_id`.
    pub idempotency_key: Option<String>,
}
#[derive(Clone, Copy)]
pub(super) enum HttpMethod {
    Get,
    Post,
    Put,
}
pub struct HttpTask {
    receiver: oneshot::Receiver<Completed>,
    abort: AbortHandle,
    finished: bool,
    completed_duration: Option<Duration>,
    completed_at: Option<web_time::Instant>,
    delivery_delay: Option<Duration>,
}
#[derive(Debug)]
struct Completed {
    result: Result<Vec<u8>, HttpError>,
    duration: Duration,
    at: web_time::Instant,
}
impl HttpTask {
    pub fn poll(&mut self) -> Option<Result<Vec<u8>, HttpError>> {
        if self.finished {
            return None;
        }
        match self.receiver.try_recv() {
            Ok(Some(completed)) => {
                self.finished = true;
                self.completed_duration = Some(completed.duration);
                self.completed_at = Some(completed.at);
                self.delivery_delay = Some(completed.at.elapsed());
                Some(completed.result)
            }
            Ok(None) => None,
            Err(_) => {
                self.finished = true;
                Some(Err(HttpError::Cancelled))
            }
        }
    }
    pub fn cancel(&self) {
        self.abort.abort();
    }
    /// Client-observed time from task creation until transport completion.
    /// It excludes any delay before the UI consumes the completed channel.
    pub fn completed_duration(&self) -> Option<Duration> {
        self.completed_duration
    }
    /// Monotonic client timestamp, available only after a completed result is polled.
    /// Never compare it with a server clock or persist it across process restarts.
    pub fn completed_at(&self) -> Option<web_time::Instant> {
        self.completed_at
    }
    /// Transport completion to the first UI poll, fixed after that poll.
    pub fn delivery_delay(&self) -> Option<Duration> {
        self.delivery_delay
    }
}
impl Drop for HttpTask {
    fn drop(&mut self) {
        self.abort.abort();
    }
}

pub struct HttpClient {
    base: url::Url,
    active: Arc<AtomicUsize>,
    #[cfg(not(target_arch = "wasm32"))]
    client: reqwest::Client,
}
struct RequestLifetime(Arc<AtomicUsize>);
impl Drop for RequestLifetime {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Release);
    }
}
impl HttpClient {
    /// Live JSON request futures for this client, including cancellation cleanup.
    /// Excludes downloads, idle pooled sockets, and completed reply buffers.
    pub fn active_requests(&self) -> usize {
        self.active.load(Ordering::Acquire)
    }
    /// Canonical API root, including any reverse-proxy path; never credentials.
    pub fn base_url(&self) -> &str {
        self.base.as_str()
    }
    pub fn new(base: &str) -> Result<Self, HttpError> {
        let mut base = url::Url::parse(base).map_err(|_| HttpError::InvalidAddress)?;
        if !matches!(base.scheme(), "http" | "https")
            || base.host_str().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(HttpError::InvalidAddress);
        }
        if !base.path().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }
        Ok(Self {
            base,
            active: Arc::new(AtomicUsize::new(0)),
            #[cfg(not(target_arch = "wasm32"))]
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| HttpError::Transport)?,
        })
    }
    pub fn start(
        &self,
        request: HttpRequest,
        ctx: &eframe::egui::Context,
    ) -> Result<HttpTask, HttpError> {
        let method = if request.json.is_some() {
            HttpMethod::Post
        } else {
            HttpMethod::Get
        };
        self.start_with_method(method, request, ctx)
    }
    pub fn put(
        &self,
        request: HttpRequest,
        ctx: &eframe::egui::Context,
    ) -> Result<HttpTask, HttpError> {
        self.start_with_method(HttpMethod::Put, request, ctx)
    }
    fn start_with_method(
        &self,
        method: HttpMethod,
        request: HttpRequest,
        ctx: &eframe::egui::Context,
    ) -> Result<HttpTask, HttpError> {
        let mut url = self.request_url(&request.path)?;
        url.query_pairs_mut().extend_pairs(&request.query);
        let ctx = ctx.clone();
        let (sender, receiver) = oneshot::channel();
        let (abort, registration) = AbortHandle::new_pair();
        let started = web_time::Instant::now();
        self.active.fetch_add(1, Ordering::Relaxed);
        let lifetime = RequestLifetime(Arc::clone(&self.active));
        #[cfg(not(target_arch = "wasm32"))]
        let client = self.client.clone();
        let future = async move {
            let _lifetime = lifetime;
            #[cfg(not(target_arch = "wasm32"))]
            let result =
                Abortable::new(native_fetch(client, url, method, request), registration).await;
            #[cfg(target_arch = "wasm32")]
            let result = Abortable::new(web::fetch(url, method, request), registration).await;
            let result = result.unwrap_or(Err(HttpError::Cancelled));
            let at = web_time::Instant::now();
            let _ = sender.send(Completed {
                result,
                duration: at.duration_since(started),
                at,
            });
            ctx.request_repaint();
        };
        #[cfg(not(target_arch = "wasm32"))]
        runtime()?.spawn(future);
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(future);
        Ok(HttpTask {
            receiver,
            abort,
            finished: false,
            completed_duration: None,
            completed_at: None,
            delivery_delay: None,
        })
    }

    fn request_url(&self, path: &str) -> Result<url::Url, HttpError> {
        if !path.starts_with('/') || path.starts_with("//") || path.contains(['?', '#', '\\']) {
            return Err(HttpError::InvalidAddress);
        }
        let url = self
            .base
            .join(path.trim_start_matches('/'))
            .map_err(|_| HttpError::InvalidAddress)?;
        if url.origin() != self.base.origin() || !url.path().starts_with(self.base.path()) {
            return Err(HttpError::InvalidAddress);
        }
        Ok(url)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn runtime() -> Result<&'static tokio::runtime::Runtime, HttpError> {
    static RUNTIME: std::sync::OnceLock<Result<tokio::runtime::Runtime, std::io::Error>> =
        std::sync::OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
        })
        .as_ref()
        .map_err(|_| HttpError::Transport)
}

#[cfg(not(target_arch = "wasm32"))]
async fn native_fetch(
    client: reqwest::Client,
    url: url::Url,
    method: HttpMethod,
    request: HttpRequest,
) -> Result<Vec<u8>, HttpError> {
    use futures::StreamExt;
    let mut builder = match (method, request.json) {
        (HttpMethod::Get, _) => client.get(url),
        (HttpMethod::Post, Some(json)) => client
            .post(url)
            .header("Content-Type", "application/json")
            .body(json),
        (HttpMethod::Put, Some(json)) => client
            .put(url)
            .header("Content-Type", "application/json")
            .body(json),
        (HttpMethod::Post, None) => client.post(url),
        (HttpMethod::Put, None) => client.put(url),
    };
    if let Some(token) = request.bearer {
        builder = builder.bearer_auth(token);
    }
    if let Some(key) = request.idempotency_key.as_deref() {
        builder = builder.header("Idempotency-Key", key);
    }
    let response = builder
        .timeout(TIMEOUT)
        .send()
        .await
        .map_err(|_| HttpError::Transport)?;
    if !response.status().is_success() {
        return Err(HttpError::Status(response.status().as_u16()));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_BODY as u64)
    {
        return Err(HttpError::TooLarge);
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| HttpError::Transport)?;
        if bytes.len().saturating_add(chunk.len()) > MAX_BODY {
            return Err(HttpError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::Instant,
    };
    #[test]
    fn completed_duration_is_fixed_when_result_is_produced_not_polled() {
        let (sender, receiver) = oneshot::channel();
        let (abort, _registration) = AbortHandle::new_pair();
        let completed_at = web_time::Instant::now() - Duration::from_secs(2);
        sender
            .send(Completed {
                result: Ok(vec![1, 2, 3]),
                duration: Duration::from_millis(37),
                at: completed_at,
            })
            .unwrap();
        let mut task = HttpTask {
            receiver,
            abort,
            finished: false,
            completed_duration: None,
            completed_at: None,
            delivery_delay: None,
        };
        assert_eq!(task.completed_duration(), None);
        assert_eq!(task.poll(), Some(Ok(vec![1, 2, 3])));
        assert_eq!(task.completed_duration(), Some(Duration::from_millis(37)));
        assert_eq!(task.completed_at(), Some(completed_at));
        let delivery = task.delivery_delay();
        assert!(delivery.is_some_and(|delay| delay >= Duration::from_secs(2)));
        assert_eq!(task.poll(), None);
        assert_eq!(task.completed_duration(), Some(Duration::from_millis(37)));
        assert_eq!(task.delivery_delay(), delivery);
    }
    fn server(
        response: &'static str,
        delay: Duration,
    ) -> (
        String,
        thread::JoinHandle<String>,
        std::sync::mpsc::Receiver<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let (accepted, ready) = std::sync::mpsc::channel();
        let thread = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
                    Err(error) => panic!("No client: {error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            let _ = accepted.send(());
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut buffer = [0u8; 1024];
            let mut request = Vec::new();
            let header_end = loop {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break request.len();
                }
                request.extend_from_slice(&buffer[..read]);
                if let Some(index) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    break index + 4;
                }
            };
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())?
                })
                .unwrap_or(0);
            while request.len() < header_end + content_length {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            thread::sleep(delay);
            let _ = stream.write_all(response.as_bytes());
            String::from_utf8(request).unwrap()
        });
        (url, thread, ready)
    }
    fn get() -> HttpRequest {
        HttpRequest {
            path: "/products".into(),
            query: vec![("brand".into(), "A&B 中文".into())],
            json: None,
            bearer: Some("fixture-token".into()),
            idempotency_key: None,
        }
    }
    fn wait(task: &mut HttpTask) -> Result<Vec<u8>, HttpError> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(result) = task.poll() {
                return result;
            }
            assert!(Instant::now() < deadline, "HTTP task did not finish");
            thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn actual_http_encodes_query_and_bearer_and_does_not_block_ui_thread() {
        let (url, server, _) = server(
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]",
            Duration::from_millis(100),
        );
        let client = HttpClient::new(&url).unwrap();
        let mut task = client.start(get(), &Default::default()).unwrap();
        assert!(task.poll().is_none());
        assert_eq!(wait(&mut task).unwrap(), b"[]");
        assert!(task.poll().is_none());
        let request = server.join().unwrap();
        assert!(request.starts_with("GET /products?brand=A%26B+%E4%B8%AD%E6%96%87 "));
        assert!(
            request
                .to_lowercase()
                .contains("authorization: bearer fixture-token")
        );
    }
    #[test]
    fn explicit_put_keeps_query_bearer_and_json_body() {
        let (url, server, _) = server(
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
            Duration::ZERO,
        );
        let client = HttpClient::new(&url).unwrap();
        let mut task = client
            .put(
                HttpRequest {
                    path: "/orders/42".into(),
                    query: vec![("audit".into(), "yes".into())],
                    json: Some(br#"{"expected_version":3}"#.to_vec()),
                    bearer: Some("fixture-token".into()),
                    idempotency_key: None,
                },
                &Default::default(),
            )
            .unwrap();
        assert_eq!(wait(&mut task).unwrap(), b"{}");
        let request = server.join().unwrap();
        assert!(request.starts_with("PUT /orders/42?audit=yes "));
        assert!(
            request
                .to_lowercase()
                .contains("content-type: application/json")
        );
        assert!(
            request
                .to_lowercase()
                .contains("authorization: bearer fixture-token")
        );
        assert!(request.ends_with("\r\n\r\n{\"expected_version\":3}"));
    }
    #[test]
    fn status_and_oversize_and_cancel_are_distinct_without_response_body_leaks() {
        for (reply, expected) in [
            (
                "HTTP/1.1 401 Unauthorized\r\nContent-Length: 11\r\nConnection: close\r\n\r\nsecret-body",
                HttpError::Status(401),
            ),
            (
                "HTTP/1.1 200 OK\r\nContent-Length: 9000000\r\nConnection: close\r\n\r\n",
                HttpError::TooLarge,
            ),
            (
                "HTTP/1.1 302 Found\r\nLocation: http://invalid.example/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                HttpError::Status(302),
            ),
        ] {
            let (url, server, _) = server(reply, Duration::ZERO);
            let mut task = HttpClient::new(&url)
                .unwrap()
                .start(get(), &Default::default())
                .unwrap();
            assert_eq!(wait(&mut task), Err(expected));
            server.join().unwrap();
        }
        let (url, server, ready) = server(
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]",
            Duration::from_millis(300),
        );
        let mut task = HttpClient::new(&url)
            .unwrap()
            .start(get(), &Default::default())
            .unwrap();
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        task.cancel();
        assert_eq!(wait(&mut task), Err(HttpError::Cancelled));
        server.join().unwrap();
    }

    #[test]
    fn twenty_cancelled_connections_close_and_release_client_tasks() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let client =
            HttpClient::new(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        for cycle in 0..20 {
            let mut task = client.start(get(), &Default::default()).unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(error) => panic!("Request did not connect: {error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = [0; 4096];
            let mut header = Vec::new();
            while !header.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                let count = stream.read(&mut bytes).unwrap();
                assert!(count > 0);
                header.extend_from_slice(&bytes[..count]);
                assert!(header.len() <= 8192);
            }
            assert_eq!(client.active_requests(), 1);
            if cycle % 2 == 0 {
                task.cancel();
                assert_eq!(wait(&mut task), Err(HttpError::Cancelled));
            }
            drop(task);
            // The server never sends a response: EOF/reset proves cancellation
            // closed the actual connection, not just a client-side task flag.
            match stream.read(&mut bytes) {
                Ok(0) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
                    ) => {}
                other => panic!("Cancelled socket remained open: {other:?}"),
            }
            let deadline = Instant::now() + Duration::from_secs(3);
            while client.active_requests() != 0 && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(2));
            }
            assert_eq!(client.active_requests(), 0);
        }
    }
}
