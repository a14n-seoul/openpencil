//! Optional embedder callbacks. The host owns resource authorization and download naming.
use js_sys::{Function, Promise, Uint8Array};
use op_editor_ui::widgets::canvas_viewport_image::{
    mark_remote_image_failed, store_remote_image_bytes, take_remote_image_requests,
};
use std::cell::{Cell, RefCell};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};

thread_local! {
    static HOST: RefCell<Option<(Function, Function)>> = const { RefCell::new(None) };
    static LOADING: Cell<usize> = const { Cell::new(0) };
}

#[wasm_bindgen]
pub fn set_embed_image_host(load_preview: Function, download: Function) {
    HOST.with(|host| *host.borrow_mut() = Some((load_preview, download)));
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
                data.dyn_into::<Uint8Array>()
            }
            .await;
            match result {
                Ok(bytes) if bytes.length() > 0 => store_remote_image_bytes(id, bytes.to_vec()),
                _ => mark_remote_image_failed(id),
            }
            LOADING.set(LOADING.get() - 1);
            crate::repaint_coalescer::request();
        });
    }
}

pub(crate) fn handle_download(name: &str, blob: &web_sys::Blob) -> Result<bool, JsValue> {
    let hook = HOST.with(|host| host.borrow().as_ref().map(|pair| pair.1.clone()));
    match hook {
        Some(hook) => Ok(hook
            .call2(&JsValue::UNDEFINED, &name.into(), blob)?
            .as_bool()
            == Some(true)),
        None => Ok(false),
    }
}
