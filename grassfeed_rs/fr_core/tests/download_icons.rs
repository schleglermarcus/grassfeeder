use fr_core::controller::contentdownloader::Downloader;
use fr_core::controller::sourcetree::SJob;
use fr_core::db::icon_repo::IIconRepo;
use fr_core::db::icon_row::IconRow;
use fr_core::db::subscription_entry::SubscriptionEntry;
use fr_core::db::subscription_repo::ISubscriptionRepo;
use fr_core::downloader::icons::IconCheckIsImage;
use fr_core::downloader::icons::IconInner;
use fr_core::downloader::icons::IconLoadStart;
use fr_core::downloader::util::retrieve_homepage_from_feed_text;
use fr_core::util::convert_webp_to_png;
use fr_core::util::Step;
use fr_core::util::StepResult;
use fr_core::web::httpfetcher::HttpFetcher;
use fr_core::web::mockfilefetcher::file_to_bin;
// use fr_core::web::mockfilefetcher::FileFetcher;
// use fr_core::web::WebFetcherType;
use fr_core::TD_BASE;
use std::io::Write;
use std::sync::Arc;

#[test]
fn image_webp_to_png() {
    setup();
    let filename = format!("{}icons/lupoca.webp", TD_BASE);
    let file_out = "../target/lupoca.png";
    let webpdata: Vec<u8> = fr_core::web::mockfilefetcher::file_to_bin(&filename).unwrap();
    let outdata = convert_webp_to_png(&webpdata, Some(20)).unwrap();
    let mut file = std::fs::File::create(file_out).unwrap();
    let w_r = file.write_all(&outdata);
    assert!(w_r.is_ok());
    // debug!("{} bytes written {:?}", outdata.len(), w_r);
    assert!(outdata.len() >= 1151 && outdata.len() <= 1288);
}

// #[ignore]
#[test]
fn multiple_icons_location() {
    setup();
    let urls: [(String, String); 10] = [
        (
            "https://www.nachdenkseiten.de/?feed=atom".to_string(),
            "".to_string(),
        ),
        (
            "http://chaosradio.ccc.de/chaosradio-complete.rss".to_string(),
            "".to_string(),
        ),
        (
            "http://www.ka-news.de/storage/rss/rss/karlsruhe.xml".to_string(),
            "http://www.ka-news.de/".to_string(),
        ),
        (
            "https://www.asue.de/rss/gesamt.xml".to_string(),
            "".to_string(),
        ),
        (
            "https://www.fromrome.info/feed/".to_string(),
            "https://www.fromrome.info/".to_string(),
        ),
        (
            "https://www.relay.fm/query/feed".to_string(),
            "https://relay-fm.relay.fm/query".to_string(), // inconsistent data delivered by website
        ),
        (
            "https://www.ft.com/news-feed?format=rss".to_string(),
            "https://www.ft.com/".to_string(),
        ),
        (
            "https://www.naturalnews.com/rss.xml".to_string(),
            "https://www.naturalnews.com/".to_string(),
        ),
        (
            "https://www.heise.de/rss/heise-atom.xml".to_string(),
            "https://www.heise.de/".to_string(),
        ),
        (
            "https://lupocattivoblog.com/feed/".to_string(),
            "https://lupocattivoblog.com/".to_string(),
        ),
        //  (            "http://feeds.seoulnews.net/rss/3f5c98640a497b43".to_string(),            "http://www.seoulnews.net".to_string(),        ),
    ];
    for u_h in urls {
        // debug!(            "multiple_icons_location:   {}   feed-url {} ",            &u_h.0, &u_h.1        );
        let (ie_list, err_happened) = download_icon_one_url(&u_h.0, &u_h.1);
        assert_eq!(ie_list.len(), 1);
        assert!(!err_happened);
    }
}

//  unstable, sometimes does not deliver a sound feed.   (XmlReader(Parser { e: EndEventMismatch { expected: "guid", found: "title" } })
fn download_icon_one_url(feed_url_: &String, homepage: &String) -> (Vec<IconRow>, bool) {
    setup();
    let se = SubscriptionEntry {
        subs_id: 1,
        url: feed_url_.clone(),
        website_url: homepage.clone(),
        ..Default::default()
    };
    let (mut icon_inner, stc_job_r) = IconInner::new_in_mem("", 1);
    icon_inner.subscriptionrepo.scrub_all_subscriptions();
    let _r = icon_inner.subscriptionrepo.store_entry(&se);
    icon_inner.web_fetcher = Arc::new(Box::new(HttpFetcher {}));
    icon_inner.feed_url = feed_url_.clone();

    let last = StepResult::start(Box::new(IconLoadStart::new(icon_inner)));
    if let Ok(ev) = stc_job_r.recv_timeout(std::time::Duration::from_millis(1)) {
        assert_eq!(ev, SJob::SetIconId(1, 1));
    }
    (
        last.iconrepo.get_all_entries(),
        last.download_error_happened,
    )
}

// #[ignore]
#[test]
fn icon_too_big() {
    setup();
    let (mut icon_inner, _stc_job_r) = IconInner::new_in_mem("", 1);
    icon_inner.feed_url = "http://lisahaven.news/feed/".to_string();
    icon_inner.web_fetcher = Arc::new(Box::new(HttpFetcher {}));

    let last = StepResult::start(Box::new(IconLoadStart::new(icon_inner)));
    assert!(!last.download_error_happened);
    let all_e = last.iconrepo.get_all_entries();
    // debug!("toobig: entries:  {:?} ", all_e);
    assert_eq!(all_e.len(), 1);
    let icon0 = all_e.get(0).unwrap();
    assert!(icon0.icon.len() < 10000);
}

// #[ignore]
#[test]
fn stop_on_nonexistent() {
    setup(); // This test issues a stop signal upon a nonexistent icon
    let (mut icon_inner, _stc_job_r) = IconInner::new_in_mem("", 5);
    icon_inner.subs_id = 5;
    let ic = IconCheckIsImage(icon_inner);
    let r: StepResult<IconInner> = Box::new(ic).step();
    assert!(matches!(r, StepResult::Stop(..)));
}

// #[ignore]
#[test]
fn test_retrieve_homepages() {
    setup();
    let files_urls: [(&str, &str); 6] = [
        ("chaosradio.xml", "https://chaosradio.de/"),
        ("nachdenkseiten-atom.xml", "https://www.nachdenkseiten.de/"),
        (
            "relay_fm_query_feed.xml",
            "https://relay-fm.relay.fm/query", // inconsistent data: this url is not available on
        ),
        ("ft_com_news_feed.xml", "https://www.ft.com/news-feed"),
        ("gorillavsbear.rss", "https://www.gorillavsbear.net/"),
        ("arstechnica_feed.rss", "https://arstechnica.com/"),
    ];
    files_urls.iter().for_each(|(f, u)| {
        let filename = format!("{}feeds/{}", TD_BASE, f);
        let buffer: Vec<u8> = file_to_bin(&filename).unwrap();
        let (hp, title, err_msg) = retrieve_homepage_from_feed_text(&buffer, "test-dl_icon");
        if hp.is_empty() {
            error!("{} {:?}", title, err_msg);
        }
        assert_eq!(hp, u.to_string());
    });
}

#[test]
fn test_retrieve_titles() {
    setup();
    let files_urls: [(&str, &str); 1] = [("linuxcomp_notitle.xml", "Linux Compatible")];
    files_urls.iter().for_each(|(f, _expected_title)| {
        let filename = format!("{}feeds/{}", TD_BASE, f);
        let buffer: Vec<u8> = file_to_bin(&filename).unwrap();
        let (_hp, title, err_msg) = retrieve_homepage_from_feed_text(&buffer, f);
        if title.is_empty() {
            error!("{} {:?}", title, err_msg);
        }
        assert_eq!(title, _expected_title.to_string());
    });
}

#[test]
fn t_host_for_url() {
    setup();
    let url = "https://www.youtube.com/feeds/videos.xml?channel_id=UC7nMSUJjOr7_TEo95Koudbg";
    let hostname = Downloader::host_from_url(&url.to_string());
    assert_eq!(hostname.unwrap(), "www.youtube.com".to_string());
}

// ------------------------------------

mod logger_config;
mod unzipper;

#[allow(unused_imports)]
#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = logger_config::setup_fern_logger(
            logger_config::QuietFlags::Downloader as u64, //  0,
        );
        unzipper::unzip_some();
    });
}
