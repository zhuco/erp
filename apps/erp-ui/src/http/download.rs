use super::*;
use futures::{
    channel::oneshot,
    future::{AbortHandle, Abortable},
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(not(target_arch = "wasm32"))]
const MAX_DOWNLOAD: u64 = 1024 * 1024 * 1024;
#[cfg(not(target_arch = "wasm32"))]
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

pub struct DownloadTask {
    receiver: oneshot::Receiver<Result<String, HttpError>>,
    abort: AbortHandle,
    progress: Arc<AtomicU64>,
    finished: bool,
    #[cfg(target_arch = "wasm32")]
    controller: web_sys::AbortController,
}

impl DownloadTask {
    pub fn poll(&mut self) -> Option<Result<String, HttpError>> {
        if self.finished {
            return None;
        }
        match self.receiver.try_recv() {
            Ok(Some(result)) => {
                self.finished = true;
                Some(result)
            }
            Ok(None) => None,
            Err(_) => {
                self.finished = true;
                Some(Err(HttpError::Cancelled))
            }
        }
    }
    pub fn cancel(&self) {
        #[cfg(target_arch = "wasm32")]
        self.controller.abort();
        self.abort.abort();
    }
    pub fn progress(&self) -> u64 {
        self.progress.load(Ordering::Relaxed)
    }
}

impl Drop for DownloadTask {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn valid_filename(filename: &str) -> bool {
    !filename.is_empty()
        && filename.len() <= 100
        && filename.is_ascii()
        && filename
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        && !filename.starts_with('.')
}

impl HttpClient {
    pub fn download(
        &self,
        request: HttpRequest,
        filename: &str,
        ctx: &eframe::egui::Context,
    ) -> Result<DownloadTask, HttpError> {
        if request.json.is_some() || !valid_filename(filename) {
            return Err(HttpError::InvalidAddress);
        }
        let mut url = self.request_url(&request.path)?;
        url.query_pairs_mut().extend_pairs(&request.query);
        let progress = Arc::new(AtomicU64::new(0));
        let task_progress = progress.clone();
        let (sender, receiver) = oneshot::channel();
        let (abort, registration) = AbortHandle::new_pair();
        let ctx = ctx.clone();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            let filename = filename.to_owned();
            let future = async move {
                let result = Abortable::new(
                    native_download(client, url, request, filename, task_progress),
                    registration,
                )
                .await
                .unwrap_or(Err(HttpError::Cancelled));
                let _ = sender.send(result);
                ctx.request_repaint();
            };
            runtime()?.spawn(future);
            Ok(DownloadTask {
                receiver,
                abort,
                progress,
                finished: false,
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            let controller = web_sys::AbortController::new().map_err(|_| HttpError::Transport)?;
            let task_controller = controller.clone();
            let filename = filename.to_owned();
            let callback_progress = task_progress.clone();
            let callback = wasm_bindgen::closure::Closure::<dyn FnMut(js_sys::BigInt)>::new(
                move |n: js_sys::BigInt| {
                    if let Some(value) = n
                        .to_string(10)
                        .ok()
                        .and_then(|value| value.as_string())
                        .and_then(|value| value.parse().ok())
                    {
                        callback_progress.store(value, Ordering::Relaxed);
                    }
                },
            );
            // Calling the JS async function creates its promise synchronously, so the
            // save picker runs in this user-gesture turn before JS reaches its first await.
            let promise = js_download(
                url.as_str(),
                request.bearer.as_deref().unwrap_or_default(),
                &filename,
                &controller,
                callback.as_ref().unchecked_ref(),
            );
            let future = async move {
                let result = Abortable::new(
                    web_download(promise, filename, callback, task_controller),
                    registration,
                )
                .await
                .unwrap_or(Err(HttpError::Cancelled));
                let _ = sender.send(result);
                ctx.request_repaint();
            };
            wasm_bindgen_futures::spawn_local(future);
            Ok(DownloadTask {
                receiver,
                abort,
                progress,
                finished: false,
                controller,
            })
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct PartialFile {
    path: std::path::PathBuf,
    armed: bool,
}
#[cfg(not(target_arch = "wasm32"))]
impl Drop for PartialFile {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn native_download(
    client: reqwest::Client,
    url: url::Url,
    request: HttpRequest,
    filename: String,
    progress: Arc<AtomicU64>,
) -> Result<String, HttpError> {
    let downloads = std::env::var_os("USERPROFILE")
        .map(std::path::PathBuf::from)
        .map(|p| p.join("Downloads"))
        .filter(|p| p.is_dir())
        .ok_or(HttpError::Transport)?;
    native_download_to(client, url, request, filename, progress, &downloads).await
}

#[cfg(not(target_arch = "wasm32"))]
async fn native_download_to(
    client: reqwest::Client,
    url: url::Url,
    request: HttpRequest,
    filename: String,
    progress: Arc<AtomicU64>,
    downloads: &std::path::Path,
) -> Result<String, HttpError> {
    use futures::StreamExt;
    use std::io::Write;
    let id = uuid::Uuid::new_v4();
    let partial_path = downloads.join(format!(".{id}-{filename}.part"));
    let final_path = downloads.join(format!("{id}-{filename}"));
    let mut guard = PartialFile {
        path: partial_path.clone(),
        armed: true,
    };
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial_path)
        .map_err(|_| HttpError::Transport)?;
    let mut builder = client.get(url).timeout(DOWNLOAD_TIMEOUT);
    if let Some(token) = request.bearer {
        builder = builder.bearer_auth(token);
    }
    let response = builder.send().await.map_err(|_| HttpError::Transport)?;
    if !response.status().is_success() {
        return Err(HttpError::Status(response.status().as_u16()));
    }
    let expected = response.content_length();
    if expected.is_some_and(|n| n > MAX_DOWNLOAD) {
        return Err(HttpError::TooLarge);
    }
    let mut stream = response.bytes_stream();
    let mut received = 0u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| HttpError::Transport)?;
        received = received
            .checked_add(chunk.len() as u64)
            .ok_or(HttpError::TooLarge)?;
        if received > MAX_DOWNLOAD {
            return Err(HttpError::TooLarge);
        }
        for part in chunk.chunks(64 * 1024) {
            file.write_all(part).map_err(|_| HttpError::Transport)?;
        }
        progress.store(received, Ordering::Relaxed);
    }
    if expected.is_some_and(|n| n != received) {
        return Err(HttpError::Transport);
    }
    file.flush().map_err(|_| HttpError::Transport)?;
    drop(file);
    // Publishing by hard link is atomic and fails if the destination somehow exists;
    // unlike rename on Unix, it can never replace an unrelated file.
    std::fs::hard_link(&partial_path, &final_path).map_err(|_| HttpError::Transport)?;
    std::fs::remove_file(&partial_path).map_err(|_| {
        let _ = std::fs::remove_file(&final_path);
        HttpError::Transport
    })?;
    guard.armed = false;
    Ok(final_path.to_string_lossy().into_owned())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(module = "/src/http/download.js")]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = erpDownload)]
    fn js_download(
        url: &str,
        token: &str,
        filename: &str,
        controller: &web_sys::AbortController,
        progress: &js_sys::Function,
    ) -> js_sys::Promise;
}

#[cfg(target_arch = "wasm32")]
async fn web_download(
    promise: js_sys::Promise,
    filename: String,
    _callback: wasm_bindgen::closure::Closure<dyn FnMut(js_sys::BigInt)>,
    controller: web_sys::AbortController,
) -> Result<String, HttpError> {
    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|error| {
            let message = error.as_string().unwrap_or_default();
            if message.contains("TOO_LARGE") {
                HttpError::TooLarge
            } else if let Some(code) = message
                .split("HTTP:")
                .nth(1)
                .and_then(|v| v.split(|c: char| !c.is_ascii_digit()).next())
                .and_then(|v| v.parse().ok())
            {
                HttpError::Status(code)
            } else if message.contains("TRANSPORT") || message.contains("SIZE_MISMATCH") {
                HttpError::Transport
            } else if message.contains("CANCELLED") || controller.signal().aborted() {
                HttpError::Cancelled
            } else {
                HttpError::Transport
            }
        })?;
    result
        .as_string()
        .filter(|value| value == &filename)
        .ok_or(HttpError::Transport)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::Instant,
    };

    fn stream_server(total: usize, pause_after_first: bool) -> (url::Url, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = url::Url::parse(&format!(
            "http://{}/erp/export",
            listener.local_addr().unwrap()
        ))
        .unwrap();
        let handle = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            let _ = socket.read(&mut request).unwrap();
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            let chunk = vec![7u8; 64 * 1024];
            let mut sent = 0;
            while sent < total {
                let count = chunk.len().min(total - sent);
                if socket.write_all(&chunk[..count]).is_err() {
                    break;
                }
                sent += count;
                if pause_after_first {
                    thread::sleep(Duration::from_millis(500));
                }
            }
        });
        (url, handle)
    }

    fn temp_downloads() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("erp-download-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        path
    }

    fn request() -> HttpRequest {
        HttpRequest {
            path: "/export".into(),
            query: vec![],
            json: None,
            bearer: None,
            idempotency_key: None,
        }
    }
    #[test]
    fn guards_paths_and_filenames_before_starting() {
        let client = HttpClient::new("http://127.0.0.1:1/erp").unwrap();
        let request = |path: &str| HttpRequest {
            path: path.into(),
            query: vec![],
            json: None,
            bearer: None,
            idempotency_key: None,
        };
        assert!(
            client
                .download(request("/../outside"), "safe.csv", &Default::default())
                .is_err()
        );
        assert!(
            client
                .download(request("/export"), "../bad.csv", &Default::default())
                .is_err()
        );
        let mut post = request("/export");
        post.json = Some(vec![]);
        assert!(
            client
                .download(post, "safe.csv", &Default::default())
                .is_err()
        );
    }

    #[test]
    fn native_streams_more_than_http_body_limit_and_keeps_finished_file() {
        let total = 9 * 1024 * 1024;
        let (url, server) = stream_server(total, false);
        let dir = temp_downloads();
        let progress = Arc::new(AtomicU64::new(0));
        let result = runtime()
            .unwrap()
            .block_on(native_download_to(
                reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                    .unwrap(),
                url,
                request(),
                "large.csv".into(),
                progress.clone(),
                &dir,
            ))
            .unwrap();
        assert_eq!(progress.load(Ordering::Relaxed), total as u64);
        assert_eq!(std::fs::metadata(&result).unwrap().len(), total as u64);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        server.join().unwrap();
        std::fs::remove_file(result).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn abort_removes_only_owned_partial() {
        let (url, server) = stream_server(16 * 1024 * 1024, true);
        let dir = temp_downloads();
        let unrelated = dir.join("keep.txt");
        std::fs::write(&unrelated, b"keep").unwrap();
        let (abort, registration) = AbortHandle::new_pair();
        let progress = Arc::new(AtomicU64::new(0));
        let task_progress = progress.clone();
        let task_dir = dir.clone();
        let future = async move {
            Abortable::new(
                native_download_to(
                    reqwest::Client::builder()
                        .redirect(reqwest::redirect::Policy::none())
                        .build()
                        .unwrap(),
                    url,
                    request(),
                    "cancel.csv".into(),
                    task_progress,
                    &task_dir,
                ),
                registration,
            )
            .await
        };
        let join = runtime().unwrap().spawn(future);
        let deadline = Instant::now() + Duration::from_secs(2);
        while progress.load(Ordering::Relaxed) == 0 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        abort.abort();
        let _ = runtime().unwrap().block_on(join).unwrap();
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        assert_eq!(std::fs::read(&unrelated).unwrap(), b"keep");
        server.join().unwrap();
        std::fs::remove_file(unrelated).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
