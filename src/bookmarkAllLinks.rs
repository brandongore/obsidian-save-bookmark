use crate::extract;
use crate::extract::Markdown;
use crate::helpers;
use crate::helpers::PersistMarkdownError;
use crate::helpers::persist_markdown;
use crate::obsidian;
use crate::settings::Settings;

use js_sys::Promise;
use js_sys::{Error, JsString};
use linkify::Link;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{future_to_promise, JsFuture};

use linkify::LinkFinder;

#[wasm_bindgen]
pub struct BookmarkAllLinksCommand {
    id: JsString,
    name: JsString,
}

#[wasm_bindgen]
impl BookmarkAllLinksCommand {
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
        future_to_promise(async move {
            let plugin = obsidian::plugin();
            let settings = crate::settings::load_settings(&plugin).await.unwrap();
            let res = bookmark_all_links(&plugin, &settings).await;
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

pub fn command_bookmarkAllLinks() -> BookmarkAllLinksCommand {
    BookmarkAllLinksCommand {
        id: JsString::from("bookmarkAllLinks"),
        name: JsString::from("Bookmark All Links"),
    }
}

async fn bookmark_all_links(
    plugin: &obsidian::Plugin,
    settings: &Settings,
) -> Result<(), PersistMarkdownError> {
    let app = plugin.app();
    let workspace = app.workspace();
    let vault = app.vault();

    if let Some(active) = workspace.get_active_file() {
        let content_js: JsString = JsFuture::from(vault.read(&active)).await?.dyn_into()?;
        let content = String::from(content_js);

        let finder = LinkFinder::new();
        let links: Vec<Link> = finder.links(&content).collect();

        for url in links {
            let msg = format!("bookmarking: {}", &url.as_str());
            obsidian::Notice::new(&msg);
            let md = url_to_markdown(&url.as_str()).await?;
            let _ = persist_markdown(settings, &vault, &md).await?;
        }
        obsidian::Notice::new(&"bookmarking complete");
        Ok(())
    } else {
        Err(PersistMarkdownError::NoActiveFile)
    }
}

async fn url_to_markdown(url: &str) -> Result<Markdown, PersistMarkdownError> {
    Ok(helpers::convert_url_to_markdown(url).await?)
}

#[wasm_bindgen(inline_js = r#" export function now() { return (+Date.now()).toString(); }"#)]
extern "C" {
    fn now() -> String;
}

