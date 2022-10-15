use feed_rs::parser;
use lz4_compression::prelude;
use std::collections::HashMap;
use tl::HTMLTag;
use tl::Node;
use url::Url;
// using   html_parser::Dom;   for extract_icon_from_homepage() creates a stack overflow

/// returns Result <   ( homepage-url, feed-title ) , error-text >
pub fn retrieve_homepage_from_feed_text(
    input: &[u8],
    dbg_feed_url: &str,
) -> Result<(String, String), String> {
    let r = parser::parse(input);
    if r.is_err() {
        return Err(format!("Parsing: {:?} {:?}", &dbg_feed_url, r.err()));
    }
    let feed = r.unwrap();
    if feed.title.is_none() {
        return Err(format!("c:title empty for {}", &dbg_feed_url));
    }
    #[allow(unused_assignments)]
    let mut feed_title: Option<String> = None;
    let mut feed_homepage: Option<String> = None;
    feed_title = Some(feed.title.unwrap().content);
    for f_link in feed.links {
        if let Some(ref mtype) = f_link.media_type {
            if mtype == "application/rss+xml" {
                continue;
            }
        }
        if let Some(ref rel) = f_link.rel {
            if rel == "hub" {
                continue;
            }
            if rel == "self" {
                continue;
            }
            if rel == "first" {
                continue;
            }
        }
        trace!(
            "   rel={:?}  href={}  type={:?}",
            &f_link.rel,
            &f_link.href,
            &f_link.media_type
        );
        feed_homepage = Some(f_link.href);
        // if !f_link.href.contains(dbg_feed_url) {            trace!("={}={}=", f_link.href, dbg_feed_url);        }
    }
    if feed_homepage.is_some() {
        return Ok((feed_homepage.unwrap(), feed_title.unwrap_or_default()));
    }
    Err(format!("no link for HP found  {} ", &dbg_feed_url))
}

/// return   Result < icon-url , error-message  >
pub fn extract_icon_from_homepage(
    hp_content: String,
    homepage_url: &String,
) -> Result<String, String> {
    let dom: tl::VDom = match tl::parse(&hp_content, tl::ParserOptions::default()) {
        Ok(d) => d,
        Err(e) => {
            return Err(format!("XI: parsing homepage: {:?}", e));
        }
    };
    let link_tags: Vec<&HTMLTag> = dom
        .nodes()
        .iter()
        .filter_map(|n| match n {
            Node::Tag(htmltag) => Some(htmltag),
            _ => None,
        })
        .filter(|htmltag| {
            let t_name = htmltag.name().as_utf8_str().into_owned();
            t_name == "link"
        })
        .collect();

    // link_tags.iter().for_each(|lt| debug!("LT:{:?}", lt));

    let icon_list: Vec<String> = link_tags
        .iter()
        .map(|t| {
            let attrmap: HashMap<String, String> = t
                .attributes()
                .iter()
                .filter(|(_k, v)| v.is_some())
                .map(|(k, v)| (k.into_owned(), v.clone().unwrap().into_owned()))
                .collect();
            attrmap
        })
        .filter(|attrmap| attrmap.get("rel").is_some())
        .filter(|attrmap| {
            if let Some(typ_e) = attrmap.get("type") {
                typ_e.contains("icon") || typ_e.starts_with("image/")
            } else {
                true
            }
        })
        // .inspect(|at_m| debug!("AM2:{:?}", at_m))
        .filter(|attrmap| attrmap.get("rel").unwrap().contains("icon"))
        .filter_map(|attrmap| attrmap.get("href").cloned())
        .collect();
    // trace!("iconlist={:?}", icon_list);
    if !icon_list.is_empty() {
        let mut icon_href: String = icon_list.get(0).unwrap().clone();
        if icon_href.starts_with("//") {
            icon_href = format!("https:{}", icon_href);
        }
        if !icon_href.starts_with("http:") && !icon_href.starts_with("https:") {
            let mut homepage_host: String = homepage_url.clone();
            if icon_href.starts_with('/') {
                match Url::parse(homepage_url) {
                    Ok(parsed) => {
                        homepage_host =
                            format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap(),);
                    }
                    Err(e) => debug!("XI:2:  ({})   ERR:{:?}", &homepage_url, e),
                }
            }
            icon_href = format!("{}{}", homepage_host, icon_href);
        }
        return Ok(icon_href);
    } else {
        return Err("no rel_icon  on page  found".to_string());
    }
}

pub fn feed_url_to_main_url(f_u: String) -> String {
    match Url::parse(&f_u) {
        Ok(parsed) => {
            let port_st = match parsed.port() {
                Some(p) => format!(":{}", p),
                None => String::default(),
            };
            let icon_url = format!(
                "{}://{}{}",
                parsed.scheme(),
                parsed.host_str().unwrap(),
                port_st
            );
            return icon_url;
        }
        Err(e) => warn!("invalid url: {}  {:?}", &f_u, e),
    }
    String::default()
}

pub fn feed_url_to_icon_url(f_u: String) -> String {
    match Url::parse(&f_u) {
        Ok(parsed) => {
            let port_st = match parsed.port() {
                Some(p) => format!(":{}", p),
                None => String::default(),
            };
            let icon_url = format!(
                "{}://{}{}/favicon.ico",
                parsed.scheme(),
                parsed.host_str().unwrap(),
                port_st
            );
            return icon_url;
        }
        Err(e) => warn!("invalid url: {}  {:?}", &f_u, e),
    }
    String::default()
}

///  Compress the data, then  encode base64  into String
pub fn compress_vec_to_string(uncompressed: &[u8]) -> String {
    let compressed_data = prelude::compress(uncompressed);
    base64::encode(compressed_data)
}

pub fn workaround_https_declaration(wrong: String) -> String {
    wrong.replace(
        "https://www.w3.org/2005/Atom",
        "http://www.w3.org/2005/Atom",
    )
}
