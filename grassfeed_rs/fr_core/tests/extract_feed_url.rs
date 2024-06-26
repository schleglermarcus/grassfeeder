mod unzipper;
use crate::unzipper::TD_BASE;
use fr_core::controller::guiprocessor::Job;
use fr_core::controller::sourcetree::SJob;
use fr_core::db::errors_repo::ErrorRepo;
use fr_core::downloader::browserdrag::extract_feed_urls_sloppy;
use fr_core::downloader::browserdrag::BrowserEvalStart;
use fr_core::downloader::browserdrag::DragInner;
use fr_core::util::StepResult;
use fr_core::web::mockfilefetcher::FileFetcher;
use fr_core::web::WebFetcherType;
use std::collections::HashMap;
use std::sync::Arc;
use xmlparser::Token;
use xmlparser::Tokenizer;

// #[ignore]
#[test]
fn t_extract_url() {
    setup();
    let html_base = format!("{}websites/", TD_BASE);
    let (stc_job_s, _stc_job_r) = flume::unbounded::<SJob>();
    let fetcher: WebFetcherType = Arc::new(Box::new(FileFetcher::new(html_base)));
    let (gp_sender, _gp_rec) = flume::bounded::<Job>(2);
    let pairs: [(&str, &str, &str); 5] = [	(
		"netjstech_com.html",
		"https://www.netjstech.com/2022/10/java-map-size-with-examples.html",
		"https://www.netjstech.com/feeds/posts/default?alt=rss",
	), 	(
		"hp_neopr.html",
		"https://www.neopresse.com/politik/teile-der-afd-fordern-atomwaffen-fuer-deutschland/",
		"https://www.neopresse.com/feed/",
	), 	(
		"pleiteticker.html",
		"https://pleiteticker.de/dkg-chef-gass-warnt-vor-winter-der-krankenhaus-insolvenzen/",
		"https://pleiteticker.de/feed/",
	),	(
		"stackexchange.html",
		"https://unix.stackexchange.com/questions/457584/gtk3-change-text-color-in-a-label-raspberry-pi",
		"https://unix.stackexchange.com/feeds/question/457584"
	),	(
        "naturalnews-page.html",
        "https://www.naturalnews.com/2022-10-22-boston-university-new-covid-kills-80-percent.html",
        "https://www.naturalnews.com/rss.xml",
    ) 	];
    for (filename, request_page, url) in pairs {
        let erro_rep = ErrorRepo::new_in_mem(); // new(&ERR_REPO_BASE.to_string());
        let mut drag_i = DragInner::new(
            filename.to_string(),
            stc_job_s.clone(),
            fetcher.clone(),
            erro_rep,
            gp_sender.clone(),
        );
        drag_i.testing_base_url = request_page.to_string();
        let last = StepResult::start(Box::new(BrowserEvalStart::new(drag_i)));
        // debug!("EX:3   '{}'  {} ", last.found_feed_url, last.error_message);
        assert_eq!(last.found_feed_url, url.to_string());
    }
}

// #[ignore]
#[test]
fn stateful_download() {
    setup();
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
    let erro_rep = ErrorRepo::new_in_mem();
    let html_base = format!("{}websites/", TD_BASE);
    let web_fetch: WebFetcherType = Arc::new(Box::new(FileFetcher::new(html_base))); // "../fr_core/tests/websites/".to_string(),
    let (gp_sender, _gp_rec) = flume::bounded::<Job>(2);
    let drag_i = DragInner::new(
        "hp_neopr.html".to_string(),
        stc_job_s.clone(),
        web_fetch.clone(),
        erro_rep,
        gp_sender,
    );
    let last = StepResult::start(Box::new(BrowserEvalStart::new(drag_i)));
    // debug!(" DL  {:?}", last.found_feed_url);
    assert_eq!(
        last.found_feed_url,
        "https://www.neopresse.com/feed/".to_string()
    );
}

// #[ignore]
#[test]
fn analyse_nn_sloppy() {
    setup();
    let fname = format!("{}{}", TD_BASE, "websites/naturalnews-page.html");
    let o_page = std::fs::read_to_string(fname.clone());
    let pagetext = o_page.unwrap();
    let found_feed_urls = extract_feed_urls_sloppy(&pagetext);
    assert_eq!(found_feed_urls.len(), 3);
}

// -------------------------------

#[derive(Default, Debug, Clone)]
struct Element {
    name: String,
    attributes: HashMap<String, String>,
}

impl std::fmt::Display for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let attrs = if self.attributes.is_empty() {
            String::default()
        } else {
            format!("{:?}", self.attributes)
        };
        write!(f, "E: {} {:?}", self.name, attrs)
    }
}

/*
The link is after an invalid comment signature.
Parsers are confused with that.

https://github.com/untitaker/html5gum
https://github.com/cloudflare/lol-html
https://github.com/servo/html5ever


// #[ignore]
// #[test]
*/
#[allow(dead_code)]
fn analyse_nn_with_html_parser() {
    setup();
    let fname = format!("{}{}", TD_BASE, "websites/naturalnews-page.html");
    let o_page = std::fs::read_to_string(fname.clone());
    let pagetext = o_page.unwrap();
    let mut tokens: Vec<Token> = Vec::default();
    for token_r in Tokenizer::from(pagetext.as_str()) {
        match token_r {
            Ok(t) => tokens.push(t),
            Err(e) => debug!("tokenizer_error {:?}", e),
        }
    }
    let mut element_list: Vec<Element> = Vec::default();
    let mut current_element = Element::default();
    for token in tokens {
        match token {
            Token::Declaration {
                version,
                encoding,
                standalone,
                span: _span,
            } => {
                debug!(
                    "Declaration: {:?} {:?} {:?} ",
                    version, encoding, standalone
                );
            }
            Token::ProcessingInstruction {
                target,
                content,
                span: _span,
            } => {
                debug!("ProcessingInstruction: {:?} {:?}  ", target, content);
            }
            Token::DtdStart {
                name,
                external_id,
                span: _span,
            } => {
                debug!("DtdStart: {:?} {:?}  ", name, external_id);
            }
            Token::EmptyDtd {
                name,
                external_id,
                span: _span,
            } => {
                trace!("EmptyDtd: {:?} {:?} ", name, external_id);
            }
            Token::Attribute {
                prefix: _p,
                local,
                value,
                span: _s,
            } => {
                current_element
                    .attributes
                    .insert(local.to_string(), value.to_string());
                //  trace!("Attribute: {:?}={:?}  ", local.to_string(), value,);
            }
            Token::ElementStart {
                prefix: _p,
                local,
                span: _s,
            } => {
                // trace!("ElementStart local:{:?}  ", local.to_string(),);
                current_element.name = local.to_string();
                current_element.attributes.clear();
            }
            Token::ElementEnd { end: _e, span: _s } => {
                //  trace!("ElementEnd: {}   ", current_element);
                element_list.push(current_element.clone());
                current_element.name = String::default();
                current_element.attributes.clear();
            }
            Token::Text { text } => {
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    // trace!("Text: {:?} ", text);
                }
            }
            Token::Cdata {
                text: _text,
                span: _s,
            } => {
                trace!("Cdata: {:?}  ", _text);
            }
            Token::Comment {
                text: _text,
                span: _s,
            } => {
                trace!("Comment: {:?}  ", _text);
            }

            _ => {
                warn!("OTHER: {:?}", token);
            }
        }

        let _filtered: Vec<&Element> = element_list
            .iter()
            .filter(|e| e.name == "link".to_string())
            .filter(|e| {
                if let Some(a_type) = e.attributes.get("type") {
                    if a_type.contains("rss") {
                        return true;
                    }
                }
                false
            })
            .inspect(|e| debug!("I0: {:?}", e.attributes))
            .filter(|e| {
                if let Some(a_rel) = e.attributes.get("rel") {
                    if a_rel.as_str() == "alternate" {
                        return true;
                    }
                }
                false
            })
            .inspect(|e| debug!("I1: {:?}", e.attributes))
            .collect::<Vec<&Element>>();

        //  for e in &element_list {            debug!("{}", e);        }
    }
}

// ------------------------------------

mod logger_config;
#[allow(unused_imports)]
#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = logger_config::setup_fern_logger(
            logger_config::QuietFlags::Downloader as u64,
            //  0,
        );
        unzipper::unzip_some();
    });
}
