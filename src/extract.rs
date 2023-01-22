use crate::helpers::ExtractError;
use crate::helpers::convert_url_to_markdown;
use crate::helpers::persist_markdown;
use crate::obsidian;
use crate::request;
use crate::settings;
use crate::helpers;
use js_sys::{Error, JsString, Promise};
use thiserror::Error;
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{future_to_promise, JsFuture};
use web_sys::console;
use web_sys::window;
use select::document::Document;
use select::predicate::Name;

#[wasm_bindgen]
pub struct ExtractCommand {
    use_clipboard: bool,
    id: JsString,
    name: JsString,
}

#[wasm_bindgen]
impl ExtractCommand {
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> JsString {
        self.id.clone()
    }

    #[wasm_bindgen(setter)]
    pub fn set_id(&mut self, id: &str) {
        self.id = JsString::from(id)
    }

    #[wasm_bindgen(getter)]
    pub fn name(&self) -> JsString {
        self.name.clone()
    }

    #[wasm_bindgen(setter)]
    pub fn set_name(&mut self, name: &str) {
        self.name = JsString::from(name)
    }

    #[wasm_bindgen(method)]
    pub fn callback(&self) -> Promise {
        let use_clipboard = self.use_clipboard;
        future_to_promise(async move {
            let plugin = obsidian::plugin();
            let res = extract_url(&plugin, use_clipboard).await;
            if let Err(e) = res {
                let msg = format!("error: {}", e);
                obsidian::Notice::new(&msg);
                Err(JsValue::from(Error::new(&msg)))
            } else {
                Ok(JsValue::undefined())
            }
        })
    }
}

pub fn command_extract_url() -> ExtractCommand {
    ExtractCommand {
        id: JsString::from("extract-url"),
        name: JsString::from("Extract"),
        use_clipboard: false,
    }
}

pub fn command_import_url() -> ExtractCommand {
    ExtractCommand {
        id: JsString::from("import-url"),
        name: JsString::from("Import From Clipboard"),
        use_clipboard: true,
    }
}



impl std::convert::From<JsValue> for ExtractError {
    fn from(err: JsValue) -> Self {
        if let Some(err_val) = err.as_string() {
            ExtractError::Fetch(format!("fetch error {}", err_val))
        } else {
            ExtractError::Fetch(String::from("fetch error"))
        }
    }
}

impl std::convert::From<obsidian::View> for ExtractError {
    fn from(_from: obsidian::View) -> Self {
        ExtractError::WrongView
    }
}

async fn read_clipboard() -> Result<String, ExtractError> {
    Ok(JsFuture::from(
        window()
            .ok_or(ExtractError::NoClipboard)?
            .navigator()
            .clipboard()
            .ok_or(ExtractError::NoClipboard)?
            .read_text(),
    )
    .await?
    .as_string()
    .ok_or(ExtractError::NoClipboardContent)?)
}

async fn extract_url(
    plugin: &obsidian::Plugin,
    use_clipboard: bool,
) -> Result<(), ExtractError> {
    if let Some(md_view) = plugin
        .app()
        .workspace()
        .get_active_view_of_type(&obsidian::MARKDOWN_VIEW)
    {
        let view: obsidian::MarkdownView = md_view.dyn_into()?;
        let editor = view.source_mode().cm_editor();
        let url_str = if use_clipboard {
            read_clipboard().await?
        } else {
            editor.get_selection()
        };
        if url_str == "" {
            Err(ExtractError::NoUrlSelected)
        } else {
            let settings = crate::settings::load_settings(&plugin).await.unwrap();
            let app = plugin.app();
            let vault = app.vault();

            let md = convert_url_to_markdown(&url_str).await?;
            persist_markdown(&settings, &vault, &md).await.or(Err(ExtractError::NoContent))?;
            Ok(())
        }
    } else {
        Err(ExtractError::WrongView)
    }
}

pub struct Markdown {
    pub title: String,
    pub content: String,
    pub available: bool
}

