//! Small browser boundary; the editor model and history are platform independent.
#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::JsCast;
    const RECOVERY_KEY: &str = "pace.editor.draft.v1";
    pub fn load() -> Result<Option<String>, String> {
        web_sys::window()
            .ok_or("Browser window unavailable")?
            .local_storage()
            .map_err(|_| "Recovery storage is unavailable")?
            .ok_or("Recovery storage is unavailable")?
            .get_item(RECOVERY_KEY)
            .map_err(|_| "Cannot read browser recovery".into())
    }
    pub fn recover(text: &str) -> Result<(), String> {
        web_sys::window().ok_or("Browser window unavailable")?.local_storage().map_err(|_| "Recovery storage is unavailable")?
            .ok_or("Recovery storage is unavailable")?.set_item(RECOVERY_KEY, text)
            .map_err(|_| "Browser recovery is full or unavailable. Save a project file now; your edits remain open.".into())
    }
    pub fn confirm(message: &str) -> bool {
        web_sys::window()
            .and_then(|w| w.confirm_with_message(message).ok())
            .unwrap_or(false)
    }
    pub fn download(text: &str) -> Result<(), String> {
        let doc = web_sys::window()
            .and_then(|w| w.document())
            .ok_or("Document unavailable")?;
        let chunks = js_sys::Array::new();
        chunks.push(&text.into());
        let mut options = web_sys::BlobPropertyBag::new();
        options.type_("application/json");
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&chunks, &options)
            .map_err(|_| "Cannot prepare project file")?;
        let url = web_sys::Url::create_object_url_with_blob(&blob)
            .map_err(|_| "Cannot prepare download")?;
        let anchor: web_sys::HtmlAnchorElement = doc
            .create_element("a")
            .map_err(|_| "Cannot prepare download")?
            .dyn_into()
            .map_err(|_| "Cannot create download link")?;
        anchor.set_href(&url);
        anchor.set_download("adventure.pace-draft.json");
        let body = doc.body().ok_or("Document body unavailable")?;
        body.append_child(&anchor)
            .map_err(|_| "Cannot attach download")?;
        anchor.click();
        anchor.remove();
        // Revoke after the browser has consumed the click, not before navigation.
        let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
            let _ = web_sys::Url::revoke_object_url(&url);
        });
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.unchecked_ref(),
                1000,
            );
        }
        Ok(())
    }
    pub fn stage_point(x: f64, y: f64) -> Option<(f64, f64)> {
        let rect = web_sys::window()?
            .document()?
            .get_element_by_id("room-stage")?
            .get_bounding_client_rect();
        if rect.width() <= 0. || rect.height() <= 0. {
            return None;
        }
        Some((
            ((x - rect.x()) / rect.width()).clamp(0., 1.),
            ((y - rect.y()) / rect.height()).clamp(0., 1.),
        ))
    }
}
#[cfg(target_arch = "wasm32")]
pub use web::*;
#[cfg(not(target_arch = "wasm32"))]
pub fn load() -> Result<Option<String>, String> {
    Ok(None)
}
#[cfg(not(target_arch = "wasm32"))]
pub fn recover(_: &str) -> Result<(), String> {
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
pub fn confirm(_: &str) -> bool {
    false
}
#[cfg(not(target_arch = "wasm32"))]
pub fn download(_: &str) -> Result<(), String> {
    Err("Downloads require the web platform".into())
}
#[cfg(not(target_arch = "wasm32"))]
pub fn stage_point(_: f64, _: f64) -> Option<(f64, f64)> {
    None
}
