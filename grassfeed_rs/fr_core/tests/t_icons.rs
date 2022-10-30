use fr_core::downloader::icons::icon_analyser;
use fr_core::util::IconKind;
use fr_core::web::mockfilefetcher;
use fr_core::TD_BASE;

/*
// #[ignore]
#[test]
fn test_asue_ico() {
    setup();
    let r = file_to_bin("tests/data/asue-favico.ico");
    let an_res = icon_analyser(&r.unwrap());
    assert_eq!(an_res.kind, IconKind::Ico);
}
*/

//RUST_BACKTRACE=1 cargo watch -s "cargo test  downloader::icons::t_::analyze_icon_local  --lib -- --exact --nocapture "
#[test]
fn analyze_icon_local() {
    setup();
    let set: [(&str, IconKind); 9] = [
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
        // trace!(            "FILE: {}   PWD={:?}",            filename,            std::env::current_dir().unwrap()        );
        let o_blob = mockfilefetcher::file_to_bin(&filename);
        if o_blob.is_err() {
            error!("{:?}  {}", &o_blob.as_ref().err(), &filename);
            panic!();
        }
        let blob = o_blob.unwrap();
        let r = icon_analyser(&blob);
        //  trace!(            "analyze_icon_local  {} \t {:?}\t{}   ",            filename, r.kind, r.message,        );
        assert_eq!(r.kind, *e_kind);
    });
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
    });
    //   unzipper::unzip_some();
    debug!("UNZIPPED: {}", unzipper::unzip_some());
}
