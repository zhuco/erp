use super::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/src/action_store/indexed_db.js")]
extern "C" {
    #[wasm_bindgen(js_name = loadCheckpoint)]
    fn load_checkpoint(key: &str) -> js_sys::Promise;
    #[wasm_bindgen(js_name = commitCheckpoint)]
    fn commit_checkpoint(key: &str, expected: f64, bytes: &[u8]) -> js_sys::Promise;
    #[wasm_bindgen(js_name = loadDraft)]
    fn load_draft_js(key: &str) -> js_sys::Promise;
    #[wasm_bindgen(js_name = commitDraft)]
    fn commit_draft_js(
        key: &str,
        expected: f64,
        bytes: &[u8],
        clear: bool,
        now: f64,
    ) -> js_sys::Promise;
}
pub(super) async fn load(key: &str) -> Result<(u64, Option<Vec<u8>>), String> {
    let value = wasm_bindgen_futures::JsFuture::from(load_checkpoint(key))
        .await
        .map_err(|_| "Cannot read browser recovery storage")?;
    let array = js_sys::Array::from(&value);
    let revision = array
        .get(0)
        .as_f64()
        .ok_or("Invalid browser recovery revision")?;
    if !revision.is_finite()
        || revision < 0.0
        || revision.fract() != 0.0
        || revision > MAX_REVISION as f64
    {
        return Err("Invalid browser recovery revision".into());
    }
    let body = array.get(1);
    if body.is_null() && revision == 0.0 {
        return Ok((0, None));
    }
    if !body.is_instance_of::<js_sys::Uint8Array>() {
        return Err("Invalid browser recovery body".into());
    }
    let bytes = js_sys::Uint8Array::new(&body);
    if bytes.length() as usize > MAX_CHECKPOINT || revision == 0.0 {
        return Err("Invalid browser recovery checkpoint".into());
    }
    Ok((revision as u64, Some(bytes.to_vec())))
}
pub(super) async fn commit(key: &str, expected: u64, bytes: &[u8]) -> Result<u64, String> {
    let value =
        wasm_bindgen_futures::JsFuture::from(commit_checkpoint(key, expected as f64, bytes))
            .await
            .map_err(
                |_| "Browser recovery write failed or conflicted; reopen storage before sending",
            )?;
    let next = value
        .as_f64()
        .ok_or("Invalid browser storage acknowledgement")?;
    if next != (expected + 1) as f64 {
        return Err("Unexpected browser recovery revision".into());
    }
    Ok(expected + 1)
}

pub(crate) async fn load_draft(key: &str) -> Result<(u64, Option<Vec<u8>>), String> {
    let value = wasm_bindgen_futures::JsFuture::from(load_draft_js(key))
        .await
        .map_err(|_| "Cannot read browser draft storage")?;
    let array = js_sys::Array::from(&value);
    let revision = array
        .get(0)
        .as_f64()
        .ok_or("Invalid browser draft revision")?;
    if !revision.is_finite()
        || revision < 0.0
        || revision.fract() != 0.0
        || revision > MAX_REVISION as f64
    {
        return Err("Invalid browser draft revision".into());
    }
    let body = array.get(1);
    if body.is_null() {
        return Ok((revision as u64, None));
    }
    if revision == 0.0 || !body.is_instance_of::<js_sys::Uint8Array>() {
        return Err("Invalid browser draft body".into());
    }
    let bytes = js_sys::Uint8Array::new(&body);
    if bytes.length() == 0 || bytes.length() as usize > crate::draft_store::MAX_DRAFT {
        return Err("Invalid browser draft snapshot".into());
    }
    Ok((revision as u64, Some(bytes.to_vec())))
}

pub(crate) async fn commit_draft(
    key: &str,
    expected: u64,
    bytes: Option<&[u8]>,
    now: u64,
) -> Result<u64, String> {
    let empty = [];
    let value = wasm_bindgen_futures::JsFuture::from(commit_draft_js(
        key,
        expected as f64,
        bytes.unwrap_or(&empty),
        bytes.is_none(),
        now as f64,
    ))
    .await
    .map_err(|_| "Browser draft write failed or conflicted; reopen drafts")?;
    let next = value
        .as_f64()
        .ok_or("Invalid browser draft storage acknowledgement")?;
    if next != (expected + 1) as f64 {
        return Err("Unexpected browser draft revision".into());
    }
    Ok(expected + 1)
}
