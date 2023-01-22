use crate::extract;
use crate::extract::Markdown;
use crate::obsidian;
use crate::request;
use crate::settings::Settings;
use fancy_regex::Regex;
use js_sys::{Error};
use lazy_static::lazy_static;
use select::document::Document;
use select::predicate::Name;
use thiserror::Error;
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture};

lazy_static! {
    pub static ref URL_REGEX: Regex =
        Regex::new(r"(?<!#\s)\[(?P<text>.*)\]\((?P<url>http.*)\)").unwrap();
    pub static ref TITLE_REGEX: Regex = Regex::new(r"[^a-zA-Z0-9\-\s]").unwrap();
}

pub struct MarkdownFile {
    title: String,
    pub file: obsidian::TFile,
}

#[derive(Error, Debug)]
pub enum PersistMarkdownError {
    #[error("expected to have a file open but none were active")]
    NoActiveFile,

    #[error("unexpected error `{0}`")]
    JsError(String),

    #[error("unknown error from js")]
    UnknownJsError,

    #[error("unknown syntax expected a url")]
    UnknownSyntax,

    #[error("error extracting content. {0}")]
    Parse(#[from] ExtractError),
}

#[derive(Error, Debug)]
pub enum ExtractError {
    #[error("url did not parse. {0}")]
    Parse(#[from] url::ParseError),

    #[error("url had not content")]
    NoContent,

    #[error("fetch error `{0}`")]
    Fetch(String),

    #[error("select a url to bookmark")]
    NoUrlSelected,

    #[error("expected view to be MarkdownView but was not")]
    WrongView,

    #[error("no clipboard available")]
    NoClipboard,

    #[error("no url in clipboard")]
    NoClipboardContent,
}


#[derive(Error, Debug)]
pub enum FrontmatterError {
    #[error("root document not a hash")]
    NotHash,

    #[error("key link not available")]
    NoLink,

    #[error("no frontmatter found in document")]
    NoYaml,

    #[error("link expected to be string type")]
    LinkNotString,
}

pub fn markdown_to_filename(md: &Markdown) -> String {
    let title = &TITLE_REGEX.replace_all(&md.title, "").replace( "\n", "");

    if let Ok(parsed) = Url::parse(&md.content) {
        if let Some(domain) = parsed.domain() {
            let no_dots = domain.replace(".", "_");
            let filename = format!("{}.{}", &no_dots, &title);
            if md.available{
                return format!("{}.md", filename)
            }
            else{
                return format!("UNAVAILABLE_{}.md", filename)
            }
        }
    }
    if md.available{
        format!("{}.md", &title)
    }
    else{
        format!("UNAVAILABLE_{}.md", &title)
    }
}

pub async fn persist_markdown(
    settings: &Settings,
    vault: &obsidian::Vault,
    md: &Markdown,
) -> Result<MarkdownFile, PersistMarkdownError> {
    let adapter = vault.adapter();
    let bookmark_path = settings.bookmark_path();
    JsFuture::from(adapter.mkdir(bookmark_path)?).await?;
    let filename = markdown_to_filename(md);
    let path = format!("{}/{}", bookmark_path, filename);
    let tfile: obsidian::TFile;
    if let Some(a_file) = vault.get_abstract_file_by_path(&path)? {
        let is_file: Result<obsidian::TFile, obsidian::TAbstractFile> = a_file.dyn_into();
        if let Ok(file) = is_file {
            tfile = file;
        } else {
            tfile = JsFuture::from(vault.create(&path, &md.content)?)
                .await?
                .dyn_into()?;
        }
    } else {
        tfile = JsFuture::from(vault.create(&path, &md.content)?)
            .await?
            .dyn_into()?;
    }
    Ok(MarkdownFile {
        title: md.title.to_owned(),
        file: tfile,
    })
}

impl std::convert::From<JsValue> for PersistMarkdownError {
    fn from(err: JsValue) -> Self {
        let err_val: &Result<Error, JsValue> = &err.dyn_into();
        if let Ok(err_val) = err_val {
            PersistMarkdownError::JsError(err_val.to_string().as_string().unwrap())
        } else {
            PersistMarkdownError::UnknownJsError
        }
    }
}

pub async fn convert_url_to_markdown(
    url: &str,
) -> Result<Markdown, ExtractError> {
    let params = request::request_params(url);

    match JsFuture::from(request::request(params)?)
    .await {
        Ok(response)=>{
            match response.as_string() {
                Some(res)=> {
                    let document = Document::from(res.as_str());
    
                    let title = document.find(Name("title")).next().unwrap();
                
                    Ok(Markdown{
                        title: title.text(),
                        content: format!("{}", url),
                        available: true
                    })
                }
                None => {
                    return Ok(Markdown{
                        title: format!("{}", url),
                        content: format!("{}", url),
                        available: false
                    })
                }
            }
        }
        Err(_)=>{
            Ok(Markdown{
                title: format!("{}", url),
                content: format!("{}", url),
                available: false
            })
        }
    }
}