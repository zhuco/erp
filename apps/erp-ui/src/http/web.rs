use super::*;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;

struct RequestGuard {
    controller: web_sys::AbortController,
    timer: i32,
    _timeout: Closure<dyn FnMut()>,
}
impl Drop for RequestGuard {
    fn drop(&mut self) {
        self.controller.abort();
        if let Some(window) = web_sys::window() {
            window.clear_timeout_with_handle(self.timer);
        }
    }
}
pub(super) async fn fetch(
    url: url::Url,
    method: HttpMethod,
    request: HttpRequest,
) -> Result<Vec<u8>, HttpError> {
    let failure = |_| HttpError::Transport;
    let window = web_sys::window().ok_or(HttpError::Transport)?;
    let controller = web_sys::AbortController::new().map_err(failure)?;
    let timeout_controller = controller.clone();
    let timeout = Closure::<dyn FnMut()>::new(move || timeout_controller.abort());
    let timer = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            timeout.as_ref().unchecked_ref(),
            TIMEOUT.as_millis() as i32,
        )
        .map_err(failure)?;
    let guard = RequestGuard {
        controller,
        timer,
        _timeout: timeout,
    };
    let options = web_sys::RequestInit::new();
    options.set_signal(Some(&guard.controller.signal()));
    options.set_redirect(web_sys::RequestRedirect::Error);
    let headers = web_sys::Headers::new().map_err(failure)?;
    if let Some(token) = request.bearer {
        headers
            .set("Authorization", &format!("Bearer {token}"))
            .map_err(failure)?;
    }
    options.set_method(match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
    });
    if let Some(json) = request.json {
        headers
            .set("Content-Type", "application/json")
            .map_err(failure)?;
        options.set_body(&js_sys::Uint8Array::from(json.as_slice()).into());
    }
    options.set_headers(&headers);
    let request =
        web_sys::Request::new_with_str_and_init(url.as_str(), &options).map_err(failure)?;
    let response: web_sys::Response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(failure)?
        .dyn_into()
        .map_err(failure)?;
    if !response.ok() {
        return Err(HttpError::Status(response.status()));
    }
    if response
        .headers()
        .get("Content-Length")
        .map_err(failure)?
        .and_then(|v| v.parse::<usize>().ok())
        .is_some_and(|length| length > MAX_BODY)
    {
        return Err(HttpError::TooLarge);
    }
    let Some(body) = response.body() else {
        return Ok(Vec::new());
    };
    let reader: web_sys::ReadableStreamDefaultReader = body
        .get_reader()
        .dyn_into()
        .map_err(|_| HttpError::Transport)?;
    let mut bytes = Vec::new();
    loop {
        let part = JsFuture::from(reader.read()).await.map_err(failure)?;
        if js_sys::Reflect::get(&part, &JsValue::from_str("done"))
            .map_err(failure)?
            .as_bool()
            == Some(true)
        {
            break;
        }
        let value = js_sys::Reflect::get(&part, &JsValue::from_str("value")).map_err(failure)?;
        let chunk = js_sys::Uint8Array::new(&value);
        if bytes.len().saturating_add(chunk.length() as usize) > MAX_BODY {
            return Err(HttpError::TooLarge);
        }
        let start = bytes.len();
        bytes.resize(start + chunk.length() as usize, 0);
        chunk.copy_to(&mut bytes[start..]);
    }
    Ok(bytes)
}
