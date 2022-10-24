// use tl::HTMLTag;
use fr_core::downloader::util::extract_feed_from_website;
use tl::Node;
const HTML_BASE: &str = "../fr_core/tests/websites/";

/* TODO Feed URL not recognized

   https://www.naturalnews.com/2022-10-22-boston-university-new-covid-kills-80-percent.html

   https://observer.ug/news/headlines/75580-uganda-land-commission-fails-to-locate-files-of-police-land-in-naguru

   Download error:
   https://insidexpress.com/lifestyle/netflix-bosses-branded-sadistic-and-wicked-for-recreating-princess-dianas-final-hours-for-the-crown/

*/
/* NO Title found:
    http://xbustyx.xxxlog.co/feed/
    https://exopolitics.blogs.com/newsinsideout/2020/08/jesus-christ-as-ascended-master-sponsored-valiant-thors-mission-from-venus-meeting-ike-russian-leaders-to-prevent-earth-nu.html
    https://www.opendesktop.org/p/1293160
    https://www.linuxcompatible.org/


*/
/*

LATER:
  https://www.reddit.com/r/Fotografie/  -> https://www.reddit.com/r/Fotografie.rss


*/

#[ignore]
#[test]
fn extract_url_work() {
    setup();
    let pairs: [(&str, &str, &str); 1] = [(
        "naturalnews-page.html",
        "https://www.naturalnews.com/2022-10-22-boston-university-new-covid-kills-80-percent.html",
        "xxxx",
    )];
    for (file, req_page, url) in pairs {
        let fname = format!("{}{}", HTML_BASE, file);
        let o_page = std::fs::read_to_string(fname.clone());
        if o_page.is_err() {
            error!("{}  {:?}", &fname, &o_page.err());
            continue;
        }
        let page = o_page.unwrap();
        let r = extract_feed_from_website(&page, &req_page);
        debug!("{:?}", r);
        assert_eq!(r, Ok(url.to_string()));
    }
}



/*

TODO:  other parser

https://github.com/untitaker/html5gum
https://github.com/cloudflare/lol-html
https://github.com/servo/html5ever



*/
#[ignore]
#[test]
fn analye_nn() {
    setup();
    let fname = format!("{}{}", HTML_BASE, "naturalnews-page.html");
    let o_page = std::fs::read_to_string(fname.clone());
    let page = o_page.unwrap();
    let dom: tl::VDom = match tl::parse(&page, tl::ParserOptions::default()) {
        Ok(d) => d,
        Err(e) => {
            error!("parsing page: {:?}", e);
            return;
        }
    };

    for node in dom.nodes() {
        match node {
            Node::Tag(_htmltag) => {
                // trace!(" {:?}", htmltag);
            }
            Node::Raw(raw) => {
                let s = String::from_utf8_lossy(&raw.as_bytes());
                let trimmed = s.trim().to_string();
                if !trimmed.is_empty() {
                    // trace!("RAW {:?}", trimmed);
                }
            }
            Node::Comment(co) => {
                trace!("COMM {:?}", co);
            } // _ => None,
        }
    }
}

#[ignore]
#[test]
fn extract_feed_urls_ok() {
    setup();
    let pairs: [(&str, &str, &str); 3] = [
        (
            "hp_neopr.html",
            "https://www.neopresse.com/politik/teile-der-afd-fordern-atomwaffen-fuer-deutschland/",
            "https://www.neopresse.com/feed/",
        ),
        (
            "pleiteticker.html",
            "https://pleiteticker.de/dkg-chef-gass-warnt-vor-winter-der-krankenhaus-insolvenzen/",
            "https://pleiteticker.de/feed/",
        ),
		(
		 "stackexchange.html",
		 "https://unix.stackexchange.com/questions/457584/gtk3-change-text-color-in-a-label-raspberry-pi",
		 "https://unix.stackexchange.com/feeds/question/457584"
	    ),

    ];

    for (file, req_page, url) in pairs {
        let fname = format!("{}{}", HTML_BASE, file);
        let page = std::fs::read_to_string(fname).unwrap();
        let r = extract_feed_from_website(&page, &req_page);
        assert_eq!(r, Ok(url.to_string()));
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
        let _r = logger_config::setup_fern_logger(0);
    });
}
