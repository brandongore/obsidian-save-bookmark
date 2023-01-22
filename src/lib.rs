mod bookmarkAllLinks;
mod extract;
mod obsidian;
mod request;
mod settings;
mod shim;
use crate::settings::*;
use js_sys::{JsString, Object, Reflect};
use thiserror::Error;
use wasm_bindgen::prelude::*;
mod helpers;

#[wasm_bindgen]
pub async fn onload(plugin: obsidian::Plugin) {
    plugin.addCommand(JsValue::from(extract::command_extract_url()));
    plugin.addCommand(JsValue::from(extract::command_import_url()));
    plugin.addCommand(JsValue::from(bookmarkAllLinks::command_bookmarkAllLinks()));
}

#[wasm_bindgen]
pub async fn settings(settings_tab: obsidian::RustSettingsTab) {
    let container = settings_tab.container_el();
    container.empty();
    if let Err(e) = settings_internal(&container).await {
        let opts = Object::new();
        Reflect::set(
            &opts,
            &JsString::from("text"),
            &JsString::from(format!("{:?}", e)),
        )
        .unwrap();
        container.create_el("p", opts.into());
    }
}

#[derive(Error, Debug)]
pub enum SettingPageError {
    #[error("error loading settings. {0}")]
    Transform(#[from] SettingsError),
}

pub async fn settings_internal<'a>(container: &obsidian::Element) -> Result<(), SettingPageError> {
    let plugin = obsidian::plugin();
    let settings = crate::settings::load_settings(&plugin).await?;
    let setting = obsidian::Setting::new(container);
    setting.set_name("bookmark path");
    setting.set_desc("location to store bookmarks");
    setting.add_text(&|text| {
        text.set_placeholder("bookmarks");
        text.set_value(&settings.bookmark_path());
        let f = Closure::wrap(Box::new(move |value| {
            wasm_bindgen_futures::spawn_local(async move {
                let plugin = obsidian::plugin();
                let mut settings = crate::settings::load_settings(&plugin).await.unwrap();
                settings.bookmark_path = Some(value);
                if let Err(e) = save_settings(&plugin, settings).await {
                    let msg = format!("error: {}", e);
                    obsidian::Notice::new(&msg);
                }
            });
        }) as Box<dyn Fn(String)>);
        text.on_change(f.into_js_value());
    });
    Ok(())
}
