use fr_core::db::icon_repo::IIconRepo;
use fr_core::db::icon_repo::IconRepo;
use fr_core::downloader::icons::icon_analyser;
use fr_core::downloader::util::extract_icon_from_homepage;
use fr_core::util::png_from_svg;
use fr_core::util::IconKind;
use fr_core::web::mockfilefetcher;
use fr_core::TD_BASE;
use std::rc::Rc;

// later: create sloppy  extract-icon-from-page for missing quotes:
//  thevaluable_dev            <link rel="shortcut icon" href=https://thevaluable.dev/images/favicon.png>
// ( "thevaluable_dev.html",   "",          "https://thevaluable.dev/images/favicon.png",                ),
#[ignore]
#[test]
fn extract_icons() {
    setup();
    //  file name inside zip,   additional-homepage,   expected icon url
    let set: [(&str, &str, &str); 8] = [
      ("naturalnews_com.html", "",
          "https://www.naturalnews.com/wp-content/themes/naturalnews-child/images/favicon.ico",           ),
      ("fromrome.html",               "",
          "https://www.fromrome.info/wp-content/uploads/2019/10/cropped-header.jpg",           ),
      ("terraherz_wpstaging.html",               "",
          "https://terraherz.wpcomstaging.com/wp-content/uploads/gwpf_icon/favicon.png",           ),
      ("terraherz_wpstaging.html",               "",
          "https://terraherz.wpcomstaging.com/wp-content/uploads/gwpf_icon/favicon.png",           ),
      ("www.seoulnews.net.html",               "",
          "https://static.themainstreammedia.com/web/newsnet/favicons/favicon.ico",           ),
      ("neweurope.html",        "",
          "https://www.neweurope.eu/wp-content/uploads/2019/07/NE-16.jpg",    ),
      ("kolkata_tv.html",   "",
          "https://s14410312.in1.wpsitepreview.link/wp-content/themes/KolkataTv/assets/images/scroll-fav.png", ),
      ( "relay_rd.html",   "https://www.relay.fm/rd",
          "https://www.relay.fm/assets/favicon-fd28d8fa5c60ac2860b452a36991933e905f82f1349c4a5ad171dd0586b2b331.ico",                ),
    ];

    set.iter().for_each(|(filename, add_url, icon_url)| {
        let fullname = format!("{}websites/{}", TD_BASE, filename);
        let page = std::fs::read_to_string(fullname).unwrap();
        let r = extract_icon_from_homepage(page, add_url);
        assert_eq!(r, Ok(icon_url.to_string()));
    });
}

#[ignore]
#[test]
fn analyze_icon_local() {
    setup();
    let set: [(&str, IconKind); 12] = [
        ("funken.svg", IconKind::Svg), // Later : add re-svg for svg conversion to bitmap
        ("slashdot-favicon.ico", IconKind::Ico), // ist en PNG    "Komprimierte Symbole werden nicht unterstützt"  ?
        ("favicon.ico", IconKind::Ico),          //
        ("icon_651.ico", IconKind::Png),         //
        ("report24-favicon.ico", IconKind::Jpg), // is jpg
        ("naturalnews_favicon.ico", IconKind::Ico),
        ("heise-safari-pinned-tab.svg", IconKind::Svg),
        ("heise-safari-pinned-tab-2024.svg", IconKind::Svg),
        ("gorillavsbear_townsquare.ico", IconKind::Ico), // MS Windows icon resource - 3 icons, 48x48, 32 bits/pixel, 48x48, 32 bits/pixel
        ("LHNN-Logo-Main-Color-1.png", IconKind::Png),
        ("seoulnews_net_favicon.ico", IconKind::Ico),
        ("asue-favico.ico", IconKind::Ico),
    ];
    set.iter().for_each(|(ic_name, e_kind)| {
        let filename = format!("{}icons/{}", TD_BASE, ic_name);
        let o_blob = mockfilefetcher::file_to_bin(&filename);
        if o_blob.is_err() {
            error!("{:?}  {}", &o_blob.as_ref().err(), &filename);
            panic!();
        }
        let blob = o_blob.unwrap();
        let r = icon_analyser(&blob);
        // trace!(            "analyze_icon_local  {} \t {:?}\t{}   ",            filename,            r.kind,            r.message,        );
        assert_eq!(r.kind, *e_kind);
    });
}

#[ignore]
#[test]
fn test_from_svg() {
    setup();
    let filename = format!("{}icons/{}", TD_BASE, "funken.svg");
    let o_blob = mockfilefetcher::file_to_bin(&filename);
    if o_blob.is_err() {
        error!("{:?}  {}", &o_blob.as_ref().err(), &filename);
        panic!();
    }
    let blob = o_blob.unwrap();
    let r = png_from_svg(&blob);
    assert!(r.is_ok());
    let r_data: Vec<u8> = r.unwrap();
    //   assert!(std::fs::write("../target/funken.png", r_data).is_ok());
    let cursor = std::io::Cursor::new(r_data);
    let decoder = png::Decoder::new(cursor);
    let mut width: u32 = 0;
    let mut height: u32 = 0;
    match decoder.read_info() {
        Ok(mut reader) => {
            let mut buf = vec![0; reader.output_buffer_size()]; // Read the next frame. An APNG might contain multiple frames.
            if let Ok(info) = reader.next_frame(&mut buf) {
                width = info.width;
                height = info.height;
            }
        }
        Err(e) => {
            warn!("png-decod {:?} ", e);
        }
    }
    assert_eq!(width, 120);
    assert_eq!(height, 120);
}

use fr_core::db::icon_repo::IconEntry;
pub const TEST_FOLDER1: &'static str = "../target/db_t_ico_rep";


/*
// cargo watch -s "(cd fr_core ;  RUST_BACKTRACE=1  cargo test  db::icon_repo::t_::t_store_file   --lib -- --exact --nocapture  )  "
#[test]
fn t_store_file() {
    setup();
    {
        debug!("t_store_file  {}  new() ... ", TEST_FOLDER1);
        let mut iconrepo = IconRepo::new_(TEST_FOLDER1);
        debug!("t_store_file  startup() ... " );
        iconrepo.startup_();
        iconrepo.clear();
        let s1 = IconEntry::default();
        assert!(iconrepo.store_entry (&s1).is_ok());
        assert!(iconrepo.store_entry(&s1).is_ok());
        let list = iconrepo.get_all_entries_();
        assert_eq!(list.len(), 2);
        iconrepo.check_or_store();
    }
    {
        let mut sr = IconRepo::new_(TEST_FOLDER1);
        sr.startup_();
        let list = sr.get_all_entries_();
        assert_eq!(list.len(), 2);
    }
}
 */


/*
// cargo watch -s "(cd fr_core ;  RUST_BACKTRACE=1  cargo test  db::icon_repo::t_::t_db_store   --lib -- --exact --nocapture  )  "
#[test]
fn t_db_store() {
    setup();
    let ir = IconRepo::new_in_mem();
    let r_ir: Rc<dyn IIconRepo> = Rc::new(ir);
    let r = (*r_ir).add_icon("hello".to_string(), 0, 0, "".to_string());
    // debug!("R: {:?} ", r);
    assert!(r.is_ok());
    let r2 = (*r_ir).get_by_index(r.unwrap()  as isize );
    assert!(r2.is_some());
    assert_eq!("hello", r2.unwrap().icon.as_str());
}
 */

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
            // logger_config::QuietFlags::Controller as u64
            0,
        );
        unzipper::unzip_some();
    });
}
