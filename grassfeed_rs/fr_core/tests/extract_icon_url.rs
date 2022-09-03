use fr_core::downloader::util::extract_icon_from_homepage;

// #[ignore]
#[test]
fn test_extract_icon_relay_rd() {
    setup();
    let f = "../fr_core/tests/data/relay_rd.html";
    let page = std::fs::read_to_string(f).unwrap();
    let r = extract_icon_from_homepage(page, &"https://www.relay.fm/rd".to_string());
    assert_eq!(
        r,
        Some(
            "https://www.relay.fm/assets/favicon-fd28d8fa5c60ac2860b452a36991933e905f82f1349c4a5ad171dd0586b2b331.ico"
                .to_string()
        )
    );
}

// #[ignore]
#[test]
fn test_extract_icon_terrahertz() {
    setup();
    let f = "../fr_core/tests/data/terraherz_wpstaging.html";
    let page = std::fs::read_to_string(f).unwrap();
    let r = extract_icon_from_homepage(page, &String::default());
    assert_eq!(
        r,
        Some(
            "https://terraherz.wpcomstaging.com/wp-content/uploads/gwpf_icon/favicon.png"
                .to_string()
        )
    );
}

// #[ignore]
#[test]
fn test_extract_icon_kolkata() {
    setup();
    let f = "../fr_core/tests/data/kolkata_tv.html";
    let page = std::fs::read_to_string(f).unwrap();
    let r = extract_icon_from_homepage(page, &String::default());
    assert_eq!(r, Some("https://s14410312.in1.wpsitepreview.link/wp-content/themes/KolkataTv/assets/images/scroll-fav.png".to_string()));
}

// #[ignore]
#[test]
fn test_extract_icon_seoul() {
    setup();
    let f = "../fr_core/tests/data/www.seoulnews.net.html";
    let page = std::fs::read_to_string(f).unwrap();
    let r = extract_icon_from_homepage(page, &String::default());
    assert_eq!(
        r,
        Some("https://static.themainstreammedia.com/web/newsnet/favicons/favicon.ico".to_string())
    );
}

// #[ignore]
#[test]
fn test_extract_icon_nn() {
    setup();
    let f = "../fr_core/tests/data/naturalnews_com.html";
    let page = std::fs::read_to_string(f).unwrap();
    let r = extract_icon_from_homepage(page, &String::default());
    assert_eq!(
        r,
        Some(
            "https://www.naturalnews.com/wp-content/themes/naturalnews-child/images/favicon.ico"
                .to_string()
        )
    );
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
        let _r = logger_config::setup_logger();
    });
}
