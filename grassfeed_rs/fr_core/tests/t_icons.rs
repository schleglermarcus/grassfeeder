use fr_core::db::icon_repo::IIconRepo;
use fr_core::db::icon_repo::IconRepo;
use fr_core::db::icon_row::CompressionType;
// use fr_core::db::icon_row::IconRow;
use fr_core::downloader::icons::icon_analyser;
use fr_core::downloader::util::extract_icon_from_homepage;
use fr_core::util::png_from_svg;
use fr_core::util::IconKind;
use fr_core::web::mockfilefetcher;
use fr_core::TD_BASE;
use resources::gen_icons;
use std::time::Instant;

// later: create sloppy  extract-icon-from-page for missing quotes:
//  thevaluable_dev            <link rel="shortcut icon" href=https://thevaluable.dev/images/favicon.png>
// ( "thevaluable_dev.html",   "",          "https://thevaluable.dev/images/favicon.png",                ),
// #[ignore]
#[test]
fn extract_icons() {
    setup();
    //  file name inside zip,   additional-homepage,   expected icon url
    let set: [(&str, &str, &str); 8] = [
      ("naturalnews-2024.html", "https://www.naturalnews.com",  "https://www.naturalnews.com/Images/favicon.ico", ),
      ("fromrome.html", "", "https://www.fromrome.info/wp-content/uploads/2019/10/cropped-header.jpg",           ),
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

// #[ignore]
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

// #[ignore]
#[test]
fn storing_solitary() {
    setup();
    let iconrepo = IconRepo::new("../target/db_icons_solitary/");
    iconrepo.create_table();
    let now = Instant::now();
    gen_icons::ICON_LIST
        .iter()
        .enumerate()
        .for_each(|(num, ico)| {
            let r = store_or_update_icon(num as isize, ico.to_string(), &iconrepo);
            assert!(r.is_ok());
        });
    // trace!("solitary, used time: {} ms ", now.elapsed().as_millis());
    assert!(now.elapsed().as_millis() < 300);
}

fn store_or_update_icon(
    id: isize,
    content: String,
    repo: &IconRepo,
) -> Result<usize, Box<dyn std::error::Error>> {
    let o_iconrow = repo.get_by_index(id);
    // trace!(        " store_default_icons : ID{} #{}  InRepo: {} ",        id,        content.len(),        o_iconrow.is_some()    );
    let result = match o_iconrow {
        Some(_r_icon) => repo.update_icon(id, Some(content), CompressionType::ImageRs),
        None => repo.store_icon(id, content, CompressionType::ImageRs),
    };
    result
}

#[test]
fn icons_store_delete_and_tx() {
    setup();
    let iconrepo = IconRepo::new("../target/db_icons_sequence/");
    let _tables_created = iconrepo.create_table();
    let now = Instant::now();
    let id_list: Vec<u8> = gen_icons::ICON_LIST
        .iter()
        .enumerate()
        .map(|(num, _i)| num as u8)
        .collect::<Vec<u8>>();
    let _num_deleted = iconrepo.delete_icons(id_list);
    let list: Vec<(isize, String)> = gen_icons::ICON_LIST
        .iter()
        .enumerate()
        .map(|(num, ic)| (num as isize, ic.to_string()))
        .collect::<Vec<(isize, String)>>();
    let r = iconrepo.store_icons_tx(list, CompressionType::None);
    // trace!("TX used time: {} ms      #deleted:{}   #tables_created:{}    R:{:?} ",        now.elapsed().as_millis(),        num_deleted,        tables_created,        r    );
    assert!(r.is_ok());
    assert_eq!(r.unwrap(), gen_icons::ICON_LIST.len());
    assert!(now.elapsed().as_millis() < 100);
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
