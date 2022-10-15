mod logger_config;

use fr_core::controller::contentdownloader::Downloader;
use fr_core::controller::sourcetree::SJob;
use fr_core::db::errors_repo::ErrorRepo;
use fr_core::db::icon_repo::IconEntry;
use fr_core::db::icon_repo::IconRepo;
use fr_core::db::subscription_entry::SubscriptionEntry;
use fr_core::db::subscription_repo::ISubscriptionRepo;
use fr_core::db::subscription_repo::SubscriptionRepo;
use fr_core::downloader::icons::IconCheckIsImage;
use fr_core::downloader::icons::IconInner;
use fr_core::downloader::icons::IconLoadStart;
use fr_core::downloader::util::retrieve_homepage_from_feed_text;
use fr_core::util::Step;
use fr_core::util::StepResult;
use fr_core::web::httpfetcher::HttpFetcher;
use fr_core::web::mockfilefetcher::file_to_bin;
use fr_core::web::mockfilefetcher::FileFetcher;
use fr_core::web::WebFetcherType;
use std::sync::Arc;

const ERRORS_FOLDER: &str = "../target/download_icons";

// #[ignore]
#[test]
fn multiple_icons_location() {
    setup();
    let urls: [(String, String); 12] = [
        (
            "http://chaosradio.ccc.de/chaosradio-complete.rss".to_string(),
            "".to_string(),
        ),
        (
            "https://www.nachdenkseiten.de/?feed=atom".to_string(),
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
            "https://www.neweurope.eu/category/world/feed/".to_string(),
            "https://www.neweurope.eu".to_string(),
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
        (
            "http://feeds.seoulnews.net/rss/3f5c98640a497b43".to_string(),
            "http://www.seoulnews.net".to_string(),
        ),
        // (            "http://chaosradio.ccc.de/chaosradio-complete.rss".to_string(),
        //     "http://chaosradio.ccc.de".to_string(),        ),
    ];
    for u_h in urls {
        // let now = std::time::Instant::now();
        let (ie_list, err_happened) = download_icon_one_url(&u_h.0, &u_h.1);
        // trace!(            "downloaded: {}  Icons:{}  err:{}  time:{}",            &u_h.0,            ie_list.len(),            err_happened,            now.elapsed().as_millis()        );
        assert_eq!(ie_list.len(), 1);
        assert!(!err_happened);
    }
}

//  unstable, sometimes does not deliver a sound feed.   (XmlReader(Parser { e: EndEventMismatch { expected: "guid", found: "title" } })
// #[ignore]
// #[test]

fn download_icon_one_url(feed_url: &String, homepage: &String) -> (Vec<IconEntry>, bool) {
    setup();
    let (stc_job_s, stc_job_r) = flume::bounded::<SJob>(9);
    let subscr_r = SubscriptionRepo::new_inmem();
    subscr_r.scrub_all_subscriptions();
    let se = SubscriptionEntry {
        subs_id: 1,
        url: feed_url.clone(),
        website_url: homepage.clone(),
        ..Default::default()
    };
    let _r = subscr_r.store_entry(&se);
    let erro_rep = ErrorRepo::new(ERRORS_FOLDER);
    erro_rep.startup_read();
    let icon_inner = IconInner {
        subs_id: 1,
        feed_url: feed_url.clone(),
        iconrepo: IconRepo::new(""),
        web_fetcher: Arc::new(Box::new(HttpFetcher {})),
        download_error_happened: false,
        icon_url: String::default(),
        icon_bytes: Vec::default(),
        fs_icon_id_old: 0,
        sourcetree_job_sender: stc_job_s,
        feed_homepage: String::default(),
        feed_download_text: String::default(),
        subscriptionrepo: subscr_r,
        erro_repo: erro_rep,
        image_icon_kind: Default::default(),
    };
    let last = StepResult::start(Box::new(IconLoadStart::new(icon_inner)));
    if let Ok(ev) = stc_job_r.recv_timeout(std::time::Duration::from_millis(1)) {
        assert_eq!(ev, SJob::SetIconId(1, 10));
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
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
    let subscr_r = SubscriptionRepo::new_inmem();
    let erro_rep = ErrorRepo::new(ERRORS_FOLDER);
    erro_rep.startup_read();
    let icon_inner = IconInner {
        subs_id: 1,
        feed_url: "http://lisahaven.news/feed/".to_string(),
        iconrepo: IconRepo::new(""),
        web_fetcher: Arc::new(Box::new(HttpFetcher {})),
        download_error_happened: false,
        icon_url: String::default(),
        icon_bytes: Vec::default(),
        fs_icon_id_old: 0,
        sourcetree_job_sender: stc_job_s,
        feed_homepage: String::default(),
        feed_download_text: String::default(),
        subscriptionrepo: subscr_r,
        erro_repo: erro_rep,
        image_icon_kind: Default::default(),
    };
    let last = StepResult::start(Box::new(IconLoadStart::new(icon_inner)));
    assert!(!last.download_error_happened);
    let all_e = last.iconrepo.get_all_entries();
    assert_eq!(all_e.len(), 1);
    let icon0 = all_e.get(0).unwrap();
    assert!(icon0.icon.len() < 10000);
}

// #[ignore]
#[test]
fn stop_on_nonexistent() {
    setup(); // This test issues a stop signal upon a nonexistant icon
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
    let subscr_r = SubscriptionRepo::new_inmem();
    let erro_rep = ErrorRepo::new(ERRORS_FOLDER);
    erro_rep.startup_read();
    let dl_inner = IconInner {
        subs_id: 5,
        feed_url: "http://localhorst/none.xml".to_string(),
        icon_url: String::default(),
        iconrepo: IconRepo::new(""),
        web_fetcher: get_file_fetcher(),
        download_error_happened: false,
        icon_bytes: Vec::default(),
        fs_icon_id_old: -1,
        sourcetree_job_sender: stc_job_s,
        feed_homepage: String::default(),
        feed_download_text: String::default(),
        subscriptionrepo: subscr_r,
        erro_repo: erro_rep,
        image_icon_kind: Default::default(),
    };
    let ic = IconCheckIsImage(dl_inner);
    let r: StepResult<IconInner> = Box::new(ic).step();
    assert!(matches!(r, StepResult::Stop(..)));
}

// #[ignore]
#[test]
fn test_retrieve_homepages() {
    setup();
    let files_urls: [(&str, &str); 6] = [
        ("tests/data/chaosradio.xml", "https://chaosradio.de/"),
        (
            "tests/data/nachdenkseiten-atom.xml",
            "https://www.nachdenkseiten.de/",
        ),
        (
            "tests/data/relay_fm_query_feed.xml",
            "https://relay-fm.relay.fm/query", // inconsistent data: this url is not available on
        ),
        (
            "tests/data/ft_com_news_feed.xml",
            "https://www.ft.com/news-feed",
        ),
        (
            "tests/data/gorillavsbear.rss",
            "https://www.gorillavsbear.net/",
        ),
        (
            "tests/data/arstechnica_feed.rss",
            "https://arstechnica.com/",
        ),
    ];
    files_urls.iter().for_each(|(f, u)| {
        let buffer: Vec<u8> = file_to_bin(f).unwrap();
        let retr_result = retrieve_homepage_from_feed_text(&buffer, "test-dl_icon");
        match retr_result {
            Ok((hp, _title)) => {
                assert_eq!(hp, u.to_string());
            }
            Err(e_d) => {
                error!("{e_d:?}");
                assert!(false);
            }
        }
    });
}

// #[ignore]
#[test]
fn t_host_for_url() {
    setup();
    let url = "https://www.youtube.com/feeds/videos.xml?channel_id=UC7nMSUJjOr7_TEo95Koudbg";
    let hostname = Downloader::host_from_url(&url.to_string());
    assert_eq!(hostname.unwrap(), "www.youtube.com".to_string());
}

fn get_file_fetcher() -> WebFetcherType {
    Arc::new(Box::new(FileFetcher::new(
        "../fr_core/tests/data/".to_string(),
    )))
}

// ------------------------------------

#[allow(unused_imports)]
#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = logger_config::setup_fern_logger(
            logger_config::QuietFlags::Downloader as u64,
            // 0,
        );
    });
}
