use fr_core::downloader::icons::icon_analyser;
use fr_core::downloader::util::extract_icon_from_homepage;
use fr_core::util::png_from_svg;
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
fn test_extract_icon_seoulnews() {
    setup();
    let filename = format!("{}websites/www.seoulnews.net.html", TD_BASE);
    let page = std::fs::read_to_string(filename).unwrap();
    let r = extract_icon_from_homepage(page, &"https://www.seoulnews.net".to_string());
    assert!(r.is_ok());
    assert_eq!(
        r.unwrap(),
        "https://static.themainstreammedia.com/web/newsnet/favicons/favicon.ico"
    );
}

/*
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
 */

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

#[test]
fn test_extract_icon_neweurop() {
    setup();
    let filename = format!("{}websites/neweurope.html", TD_BASE);
    let page = std::fs::read_to_string(filename).unwrap();
    let r = extract_icon_from_homepage(page, &"https://www.neweurope.eu/".to_string());
    assert_eq!(
        r,
        Ok("https://www.neweurope.eu/wp-content/uploads/2019/07/NE-16.jpg".to_string())
    );
}

#[test]
fn test_extract_icon_kolkata() {
    setup();
    let filename = format!("{}websites/{}", TD_BASE, "kolkata_tv.html");
    let page = std::fs::read_to_string(filename).unwrap();
    let r = extract_icon_from_homepage(page, &String::default());
    assert_eq!(r, Ok("https://s14410312.in1.wpsitepreview.link/wp-content/themes/KolkataTv/assets/images/scroll-fav.png".to_string()));
}

#[test]
fn test_extract_icon_nn() {
    setup();
    let filename = format!("{}websites/naturalnews_com.html", TD_BASE);
    let page = std::fs::read_to_string(filename).unwrap();
    let r = extract_icon_from_homepage(page, &String::default());
    assert_eq!(
        r,
        Ok(
            "https://www.naturalnews.com/wp-content/themes/naturalnews-child/images/favicon.ico"
                .to_string()
        )
    );
}

// #[ignore]
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
        // trace!("-->file: {}   ", filename);
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
    // let r = std::fs::write("../target/funken.png", r_data);    assert!(r.is_ok());
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
