//! Optional embedder callbacks. The host owns resource authorization and download naming.
use js_sys::{Function, Promise, Uint8Array};
use op_editor_ui::widgets::canvas_viewport_image::{
    mark_remote_image_failed, release_remote_image_request, store_remote_image_bytes,
    take_remote_image_requests,
};
use std::cell::{Cell, RefCell};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};

thread_local! {
    static HOST: RefCell<Option<(Function, Function)>> = const { RefCell::new(None) };
    static EXPORT: RefCell<Option<Function>> = const { RefCell::new(None) };
    static LOADING: Cell<usize> = const { Cell::new(0) };
}

#[wasm_bindgen]
pub fn set_embed_image_host(load_preview: Function, download: Function) {
    HOST.with(|host| *host.borrow_mut() = Some((load_preview, download)));
}

/// Optional browser-local encoder. Standalone builds keep the daemon export path.
#[wasm_bindgen]
pub fn set_embed_export_host(export: Function) {
    EXPORT.with(|slot| *slot.borrow_mut() = Some(export));
}

pub(crate) fn export_hook() -> Option<Function> {
    EXPORT.with(|slot| slot.borrow().clone())
}

pub(crate) fn drain() {
    let Some(loader) = HOST.with(|host| host.borrow().as_ref().map(|pair| pair.0.clone())) else {
        return;
    };
    for (id, url) in take_remote_image_requests(4usize.saturating_sub(LOADING.get())) {
        LOADING.set(LOADING.get() + 1);
        let loader = loader.clone();
        spawn_local(async move {
            let result = async {
                let pending = loader.call1(&JsValue::UNDEFINED, &url.into())?;
                let data = JsFuture::from(Promise::resolve(&pending)).await?;
                Ok::<_, JsValue>(data.dyn_into::<Uint8Array>().ok())
            }
            .await;
            match result {
                Ok(Some(bytes)) if bytes.length() > 0 => {
                    store_remote_image_bytes(id, bytes.to_vec())
                }
                Ok(_) => mark_remote_image_failed(id),
                Err(_) => {
                    // Leave this ID in flight through the cooldown. Only the next visible paint
                    // may requeue it, so navigating away does not keep a retry loop alive.
                    let retry = Closure::once_into_js(move || {
                        release_remote_image_request(id);
                        crate::repaint_coalescer::request();
                    });
                    if web_sys::window()
                        .and_then(|w| {
                            w.set_timeout_with_callback_and_timeout_and_arguments_0(
                                retry.unchecked_ref(),
                                30_000,
                            )
                            .ok()
                        })
                        .is_none()
                    {
                        mark_remote_image_failed(id);
                    }
                }
            }
            LOADING.set(LOADING.get() - 1);
            crate::repaint_coalescer::request();
        });
    }
}

pub(crate) fn handle_download(
    name: &str,
    blob: &web_sys::Blob,
) -> Result<Option<Promise>, JsValue> {
    let hook = HOST.with(|host| host.borrow().as_ref().map(|pair| pair.1.clone()));
    match hook {
        Some(hook) => Ok(hook
            .call2(&JsValue::UNDEFINED, &name.into(), blob)?
            .dyn_into::<Promise>()
            .ok()),
        None => Ok(None),
    }
}
