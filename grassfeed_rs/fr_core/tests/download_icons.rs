mod logger_config;

use fr_core::controller::contentdownloader::Downloader;
use fr_core::controller::sourcetree::SJob;
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

//  unstable, sometimes does not deliver a sound feed.   (XmlReader(Parser { e: EndEventMismatch { expected: "guid", found: "title" } })
// #[ignore]
#[test]
#[allow(dead_code)]
fn icon_dl_naturalnews() {
    setup();
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
    let subscr_r = SubscriptionRepo::new_inmem();
    subscr_r.scrub_all_subscriptions();
    let se = SubscriptionEntry {
        subs_id: 1,
        url: "https://www.naturalnews.com/rss.xml".to_string(),
        website_url: "https://www.naturalnews.com/".to_string(),
        ..Default::default()
    };
    let _r = subscr_r.store_entry(&se);

    //    let subscriptionrepo = SubscriptionRepo::by_existing_connection(subscr_r.get_connection());

    // tODO store   subscription before

    let icon_inner = IconInner {
        fs_repo_id: 1,
        feed_url: "https://www.naturalnews.com/rss.xml".to_string(),
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
    };
    let last = StepResult::start(Box::new(IconLoadStart::new(icon_inner)));
    assert!(!last.download_error_happened);
    let all_e = last.iconrepo.get_all_entries();

    // for e in &all_e {        debug!("R: {:?}", &e);    }
    assert_eq!(all_e.len(), 1);

    // let se1 = last.subscriptionrepo.get_by_index(1).unwrap();    debug!("SE1={:?}", &se1);
}

// #[ignore]
#[test]
fn icon_download_heise() {
    setup();
    let (stc_job_s, stc_job_r) = flume::bounded::<SJob>(9);
    let subscr_r = SubscriptionRepo::new_inmem();
    let icon_inner = IconInner {
        fs_repo_id: 1,
        feed_url: "https://www.heise.de/rss/heise-atom.xml".to_string(),
        iconrepo: IconRepo::new(""),
        web_fetcher: Arc::new(Box::new(HttpFetcher {})),
        download_error_happened: false,
        icon_url: "favicon.ico".to_string(),
        icon_bytes: Vec::default(),
        fs_icon_id_old: 0,
        sourcetree_job_sender: stc_job_s,
        feed_homepage: String::default(),
        feed_download_text: String::default(),
        subscriptionrepo: subscr_r,
    };
    let last = StepResult::start(Box::new(IconLoadStart::new(icon_inner)));
    assert!(!last.download_error_happened);
    let all_e = last.iconrepo.get_all_entries();
    assert_eq!(all_e.len(), 1);
    assert_eq!(stc_job_r.recv(), Ok(SJob::SetIconId(1, 10)));
}

// #[ignore]
#[test]
fn t_host_for_url() {
    setup();
    let url = "https://www.youtube.com/feeds/videos.xml?channel_id=UC7nMSUJjOr7_TEo95Koudbg";
    let hostname = Downloader::host_from_url(&url.to_string());
    assert_eq!(hostname.unwrap(), "www.youtube.com".to_string());
}

// #[ignore]
#[test]
fn t_iconcheck_isimage() {
    setup();
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
    let subscr_r = SubscriptionRepo::new_inmem();
    let dl_inner = IconInner {
        fs_repo_id: 5,
        feed_url: "http://feeds.seoulnews.net/rss/3f5c98640a497b43".to_string(),
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
    };
    let ic = IconCheckIsImage(dl_inner);
    let r: StepResult<IconInner> = Box::new(ic).step();
    assert!(matches!(r, StepResult::Stop(..)));
}

// #[ignore]
#[test]
fn icon_lupocatt() {
    setup();
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
    let subscr_r = SubscriptionRepo::new_inmem();
    let icon_inner = IconInner {
        fs_repo_id: 1,
        feed_url: "https://lupocattivoblog.com/feed/".to_string(),
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
    };
    let last = StepResult::start(Box::new(IconLoadStart::new(icon_inner)));
    assert!(!last.download_error_happened);
    let all_e = last.iconrepo.get_all_entries();
    assert_eq!(all_e.len(), 1);
}

// #[ignore]
#[test]
fn icon_simple_chaosradio() {
    setup();
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
    let subscr_r = SubscriptionRepo::new_inmem();
    let icon_inner = IconInner {
        fs_repo_id: 1,
        feed_url: "http://chaosradio.ccc.de/chaosradio-complete.rss".to_string(),
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
    };
    let last = StepResult::start(Box::new(IconLoadStart::new(icon_inner)));
    assert!(!last.download_error_happened);
    let all_e = last.iconrepo.get_all_entries();
    assert_eq!(all_e.len(), 1);
}

// The Feed cannot be parsed  -> unstable
// #[ignore]
#[test]
// #[allow(dead_code)]
fn icon_simple_seoulnews() {
    setup();
    let icon_repo = IconRepo::new("");
    let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(9);
    let subscr_r = SubscriptionRepo::new_inmem();
    let icon_inner = IconInner {
        fs_repo_id: 1,
        feed_url: "http://feeds.seoulnews.net/rss/3f5c98640a497b43".to_string(),
        iconrepo: icon_repo,
        web_fetcher: Arc::new(Box::new(HttpFetcher {})),
        download_error_happened: false,
        icon_url: String::default(),
        icon_bytes: Vec::default(),
        fs_icon_id_old: 0,
        sourcetree_job_sender: stc_job_s,
        feed_homepage: String::default(),
        feed_download_text: String::default(),
        subscriptionrepo: subscr_r,
    };
    let last = StepResult::start(Box::new(IconLoadStart::new(icon_inner)));
    assert!(!last.download_error_happened);
    let all_e = last.iconrepo.get_all_entries();
    assert_eq!(all_e.len(), 1);
}

// #[ignore]
#[test]
fn test_retrieve_homepages() {
    setup();
    let files_urls: [(&str, &str); 4] = [
        (
            "tests/data/gorillavsbear.rss",
            "https://www.gorillavsbear.net/",
        ),
        (
            "tests/data/arstechnica_feed.rss",
            "https://arstechnica.com/",
        ),
        (
            "tests/data/chaosradio-complete.rss",
            "https://chaosradio.de/",
        ),
        ("tests/data/relay_rd.rss", "https://www.relay.fm/rd"),
    ];
    files_urls.iter().for_each(|(f, u)| {
        let buffer: Vec<u8> = file_to_bin(f).unwrap();
        let (o_hp, _o_title) = retrieve_homepage_from_feed_text(&buffer, "various");
        assert_eq!(o_hp, Some(u.to_string()));
    });
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
            	logger_config::QuietFlags::Downloader as u64
            // 0,
        );
    });
}
