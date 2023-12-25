use fr_core::downloader::icons::icon_analyser;
use fr_core::downloader::util::extract_icon_from_homepage;
use fr_core::util::downscale_image;
use fr_core::util::IconKind;
use fr_core::web::mockfilefetcher;
use fr_core::TD_BASE;

// #[ignore]
#[test]
fn test_extract_icon_fromrome() {
    setup();
    let filename = format!("{}websites/fromrome.html", TD_BASE);
    let page = std::fs::read_to_string(filename).unwrap();
    let r = extract_icon_from_homepage(page, &"https://www.fromrome.info/".to_string());
    assert_eq!(
        r,
        Ok("https://www.fromrome.info/wp-content/uploads/2019/10/cropped-header.jpg".to_string())
    );
}

// #[ignore]
#[test]
fn test_extract_icon_seoul() {
    setup();
    let filename = format!("{}websites/www.seoulnews.net.html", TD_BASE);
    let page = std::fs::read_to_string(filename).unwrap();
    let r = extract_icon_from_homepage(page, &String::default());
    assert_eq!(
        r,
        Ok("https://static.themainstreammedia.com/web/newsnet/favicons/favicon.ico".to_string())
    );
}

// #[ignore]
#[test]
fn test_extract_icon_terrahertz() {
    setup();
    let filename = format!("{}websites/terraherz_wpstaging.html", TD_BASE);
    let page = std::fs::read_to_string(filename).unwrap();
    let r = extract_icon_from_homepage(page, &String::default());
    assert_eq!(
        r,
        Ok(
            "https://terraherz.wpcomstaging.com/wp-content/uploads/gwpf_icon/favicon.png"
                .to_string()
        )
    );
}

// #[ignore]
#[test]
fn test_extract_icon_relay_rd() {
    setup();
    let filename = format!("{}websites/relay_rd.html", TD_BASE);
    let page = std::fs::read_to_string(filename).unwrap();
    let r = extract_icon_from_homepage(page, &"https://www.relay.fm/rd".to_string());
    assert_eq!(
        r,
        Ok(
            "https://www.relay.fm/assets/favicon-fd28d8fa5c60ac2860b452a36991933e905f82f1349c4a5ad171dd0586b2b331.ico"
                .to_string()
        )
    );
}

//RUST_BACKTRACE=1 cargo watch -s "cargo test  downloader::icons::t_::analyze_icon_local  --lib -- --exact --nocapture "
//  #[ignore]
#[test]
fn analyze_icon_local() {
    setup();
    let set: [(&str, IconKind); 9] = [
        //         ("funken.svg", IconKind::Svg),      // Later : add re-svg for svg conversion to bitmap
        ("favicon.ico", IconKind::Ico),          //
        ("icon_651.ico", IconKind::Png),         //
        ("report24-favicon.ico", IconKind::Jpg), // is jpg
        ("naturalnews_favicon.ico", IconKind::Ico),
        ("heise-safari-pinned-tab.svg", IconKind::Svg),
        ("gorillavsbear_townsquare.ico", IconKind::Ico), // MS Windows icon resource - 3 icons, 48x48, 32 bits/pixel, 48x48, 32 bits/pixel
        ("LHNN-Logo-Main-Color-1.png", IconKind::Png),
        ("seoulnews_favicon.ico", IconKind::UnknownType),
        ("asue-favico.ico", IconKind::Ico),
    ];
    set.iter().for_each(|(ic_name, e_kind)| {
        let filename = format!("{}icons/{}", TD_BASE, ic_name);
        trace!("FILE: {}   ", filename);
        let o_blob = mockfilefetcher::file_to_bin(&filename);
        if o_blob.is_err() {
            error!("{:?}  {}", &o_blob.as_ref().err(), &filename);
            panic!();
        }
        let blob = o_blob.unwrap();
        let r = icon_analyser(&blob);
        trace!(
            "analyze_icon_local  {} \t {:?}\t{}   ",
            filename,
            r.kind,
            r.message,
        );
        assert_eq!(r.kind, *e_kind);
    });
}

#[ignore] // later re-svg
#[test]
fn t_downscale_icon() {
    setup();
    let filename = format!("{}icons/{}", TD_BASE, "funken.svg");
    let o_blob = mockfilefetcher::file_to_bin(&filename);
    if o_blob.is_err() {
        error!("{:?}  {}", &o_blob.as_ref().err(), &filename);
        panic!();
    }
    let blob = o_blob.unwrap();
    let r = downscale_image(&blob, &IconKind::Svg, 64);
    debug!("R={:?} ", r);
    assert!(r.is_ok());
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
        let _r = logger_config::setup_fern_logger(logger_config::QuietFlags::Controller as u64);
        unzipper::unzip_some();
    });
}
