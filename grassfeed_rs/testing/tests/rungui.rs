use chrono::Local;
use chrono::TimeZone;
use context::appcontext::AppContext;
use fr_core::config::prepare_ini::GrassFeederConfig;
use fr_core::controller::sourcetree::ISourceTreeController;
use fr_core::controller::sourcetree::SourceTreeController;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use fr_core::db::subscription_repo::ISubscriptionRepo;
use fr_core::db::subscription_repo::SubscriptionRepo;
use resources::loc;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;
use testing::minihttpserver::minisrv::MiniHttpServerController;
use testing::minihttpserver::minisrv::ServerConfig;

#[macro_use]
extern crate rust_i18n;

i18n!("../resources/locales");

const MINIHTTPSERVER_PORT: usize = 8123;
const RSS_DYNAMIC_FILENAME: &str = "../target/dynamic.rss";

fn startup_minihttpserver(port: usize) -> MiniHttpServerController {
    let conf = ServerConfig {
        htdocs_dir: String::from("tests/fr_htdocs"),
        index_file: String::from("index.html"),
        tcp_address: format!("127.0.0.1:{}", port).to_string(),
        binary_max_size: 1000000,
        download_throttling_kbps: 10,
    };
    let mut msc = MiniHttpServerController::new(Arc::new(conf));
    msc.start();
    msc
}

fn entry(title: &str, link: &str, descr: &str, pubdate: i64) -> String {
    let pubdate_str = chrono::offset::Local.timestamp(pubdate, 0).to_rfc2822();
    format!(
        " <item>\n   <title>{}</title>\n   <link>{}</link>
		<description>{}</description>
		<pubDate>{}</pubDate>\n  </item>\n ",
        title, link, descr, pubdate_str
    )
}

fn write_feed() {
    setup();
    let header = "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>
<rss version=\"2.0\">
 <channel>
  <title>Dynamically created</title>
  <description>lorem ipsum</description> \n";
    let footer = "\n </channel>\n</rss> \n";
    let ts_now = Local::now().timestamp();
    let mut file = File::create(RSS_DYNAMIC_FILENAME).unwrap();
    file.write(header.as_bytes()).unwrap();
    let entryline = entry(
        format!("TITLE-{}", ts_now).as_str(),
        "link",
        "description",
        ts_now,
    );
    file.write(entryline.as_bytes()).unwrap();
    file.write(footer.as_bytes()).unwrap();
    // debug!("written to {} {}", RSS_DYNAMIC_FILENAME, ts_now);
}

#[ignore]
#[test]
fn rungui_local_clear() {
    setup();
    loc::init_locales();
    let mut mini_server_c = startup_minihttpserver(MINIHTTPSERVER_PORT);
    let _dyn_wr_handle = std::thread::spawn(|| loop {
        write_feed();
        std::thread::sleep(std::time::Duration::from_secs(59));
    });

    let gfconf = GrassFeederConfig {
        path_config: "../target/db_rungui_local".to_string(),
        path_cache: "../target/db_rungui_local".to_string(),
        debug_mode: false,
    };
    let appcontext = fr_core::grassfeeder::start(gfconf);
    test_setup_values(&appcontext, mini_server_c.get_address());
    fr_core::grassfeeder::run(&appcontext);

    mini_server_c.stop();
}

fn test_setup_values(acr: &AppContext, addr: String) {
    let fsrwr: Rc<RefCell<dyn ISubscriptionRepo>> = acr.get_rc::<SubscriptionRepo>().unwrap();
    (*fsrwr.borrow()).clear();
    if false {
        let msg_r: Rc<RefCell<dyn IMessagesRepo>> = acr.get_rc::<MessagesRepo>().unwrap();
        let _r = (*msg_r).borrow().get_ctx().delete_table();
        (*msg_r).borrow().get_ctx().create_table();
    }

    let feedsources_r: Rc<RefCell<dyn ISourceTreeController>> =
        acr.get_rc::<SourceTreeController>().unwrap();
    let ref mut feedsources = (*feedsources_r).borrow_mut();

    let url_dynamic = format!("{}/dynamic.rss", addr);
    let url_gui_proc = format!("{}/gui_proc_2.rss", addr);
    let url_feedburner = format!("{}/feedburner.rss", addr);
    let url_staseve = format!("{}/staseve-11.xml", addr);
    let url_r_foto = format!("{}/reddit-Fotografie.rss", addr);
    let url_insi = format!("{}/newsinsideout_com.rss", addr);
    // let _url_dieneuewelle = format!("{}/dieneuewelle.xml", addr);
    let url_nn_aug = format!("{}/naturalnews_aug.xml", addr);

    let f_hill = feedsources.add_new_folder_at_parent("hill".to_string(), 0);
    let src = [
        //	(url_dieneuewelle.as_str(), "dieneuewelle, ampersand")
        (url_nn_aug.as_str(), "NN-aug, special chars"),
    ];
    src.iter().for_each(|(url, desc)| {
        feedsources.add_new_feedsource_at_parent(url.to_string(), desc.to_string(), f_hill, false);
    });
    let folder2 = feedsources.add_new_folder_at_parent("folder2".to_string(), 0);
    if true {
        let src = [
            (url_dynamic.as_str(), "dynamic"),
            (url_gui_proc.as_str(), "gui_proc_2"),
            (url_staseve.as_str(), "staseve11"),
        ];
        src.iter().for_each(|(url, desc)| {
            feedsources.add_new_feedsource_at_parent(
                url.to_string(),
                desc.to_string(),
                folder2,
                false,
            );
        });

        let folder3 = feedsources.add_new_folder_at_parent("folder3".to_string(), 0);
        let src = [
            (url_r_foto.as_str(), "fotograf"),
            (url_feedburner.as_str(), "feedburner"),
            (url_insi.as_str(), "newsinsideout_com"),
            ("https://www.naturalnews.com/rss.xml", "naturalnews.com"),
            ("http://www.peopleofwalmart.com/feed/", "walmart"), // why error ?
        ];
        src.iter().for_each(|(url, desc)| {
            feedsources.add_new_feedsource_at_parent(
                url.to_string(),
                desc.to_string(),
                folder3,
                false,
            );
        });
    }
    if false {
        let src = [
            ("http://rss.slashdot.org/Slashdot/slashdot", "slashdot"), // sometimes delivers 403
            ("https://www.blacklistednews.com/rss.php", "blacklisted"), // hour-minute-seconds are all set to 0
            ("https://xkcd.com/atom.xml", "Xkcd-no-pubdate"),
            ("https://www.asue.de/rss/gesamt.xml", "asue-no-pubdate"),
            ("http://feeds.bbci.co.uk/news/rss.xml", "bbc"),
            ("https://www.neweurope.eu/category/world/feed/", "neweurope"),
            ("https://www.headlinesoftoday.com/feed", "headlinesof"),
            ("https://insidexpress.com/feed/", "insidexpress"),
            ("https://linuxnews.de/feed/", "linuxnews"),
            ("https://linuxnews.de/comments/feed/", "linuxnews-comm"),
            ("https://www.linuxtoday.com/feed/", "linuxtoday"),
            ("https://news.itsfoss.com/feed/", "itsfoss"),
            ("https://www.buzzfeed.com/world.xml", "buzzfeed"),
            ("https://www.ft.com/news-feed?format=rss", "financialtimes"),
            ("https://www.openpr.de/rss/openpr.xml", "openpr"),
            ("https://www.reddit.com/r/funny.rss", "reddit-funny"),
            ("https://www.reddit.com/r/gaming.rss", "reddit-gaming"),
            ("https://tickernews.co/feed/", "tickernews"),
            ("https://tickernews.co/comments/feed/", "tickernews-comm"),
            ("https://www.gawker.com/rss", "gawker"),
            ("https://arcadi-online.de/feed/", "arcadi"),
            ("http://thehighersidechats.com/feed/", "higherside"),
            ("https://www.kino.de/rss/neu-im-kino", "kino_neu"),
            ("https://kolkatatv.org/feed/", "kolkatatv"),
            ("https://lupocattivoblog.com/feed/", "lupocat"),
            ("http://www.wissensmanufaktur.net/rss.xml", "wissensm"),
            ("http://feeds.feedburner.com/blogspot/cwWR", "financearmag"),
            ("https://opposition24.com/feed/", "opposition"),
            ("http://newsinsideout.com/feed/", "newsinsideout"),
            ("https://sciencefiles.org/feed/", "science"),
            ("http://www.watergate.tv/feed/", "watergate"),
            ("http://n8waechter.info/feed/", "na8wae"),
            ("http://www.guidograndt.de/feed/", "guido"),
            ("https://readrust.net/all/feed.rss", "readrust"),
            ("https://www.relay.fm/rd/feed", "rel_rd"),
            ("https://www.relay.fm/query/feed", "rel_query"),
            ("http://feeds.feedburner.com/euronews/en/news/", "euronews"),
            ("https://www.ksta.de/feed/index.rss", "koelner"),
            ("https://kodansha.us/feed/", "Kodansha"),
            ("https://planet.debian.org/rss20.xml", "debian"),
            ("https://report24.news/feed/", "report24"),
            ("https://www.heise.de/rss/heise-atom.xml", "heise-atom"),
            ("https://www.theugandatoday.com/feed/", "theuganda"),
            ("https://de.rt.com/feeds/news/", "RT DE"),
            ("https://terraherz.wpcomstaging.com/feed/", "terraherz"),
            ("https://www.reddit.com/r/aww.rss", "aww"),
            ("https://feeds.breakingnews.ie/bnworld", "breaknew"),
        ];

        let folder3 = feedsources.add_new_folder_at_parent("folder3".to_string(), 0);
        src.iter().for_each(|(url, desc)| {
            feedsources.add_new_feedsource_at_parent(
                url.to_string(),
                desc.to_string(),
                folder3,
                false,
            );
        });
    }
    if false {
        let src = [
            (
                "http://www.tagesschau.de/newsticker.rdf",
                "tagesschau-no-pubdate",
            ),
			(
                "https://www.thenexthint.com/feed/",
                "nexthint 無料ダウンロード",
            ),
            (
                "https://packages.gentoo.org/packages/added.atom",
                "gentoo-added_no-pubdate",
            ), //  pubDate not there, but <updaed>
            (
                "https://observer.ug/headlinenews?format=feed&type=rss",
                "obs_uganda",
            ),
            (
                "https://www.youtube.com/feeds/videos.xml?channel_id=UCzUV5283-l5c0oKRtyenj6Q",
                "MarkDice",
            ),
            (
                "https://www.youtube.com/feeds/videos.xml?channel_id=UCTiL1q9YbrVam5nP2xzFTWQ",
                "SuspiciousObservers",
            ),
            (
                "https://www.newscentric.com.ng/feeds/posts/default?alt=rss",
                "ncentric",
            ),
            (
                "https://allworldnews24hours6.blogspot.com/feeds/posts/default",
                "allworld24",
            ),
            (
                "https://www.newsrust.com/feeds/posts/default?alt=rss",
                "newsrust",
            ),
            (
                "http://www.channelnewsasia.com/rssfeeds/8395884",
                "newsasia",
            ),
            (
                "https://www.euronews.com/rss?level=theme&name=news",
                "euronews",
            ),
            (
                "http://www.marketwatch.com/rss/realtimeheadlines",
                "marketwatch",
            ),
            (
                "http://feeds.feedburner.com/RichardHerringLSTPodcast",
                "RichardHerring",
            ),
            (
                "https://www.gorillavsbear.net/category/mp3/feed/",
                "gorilla",
            ),
            (
                "http://rss.cnn.com/rss/edition_entertainment.rs",
                "cnn_entertain",
            ),
            (
                "http://www.ka-news.de/storage/rss/rss/karlsruhe.xml",
                "ka-news",
            ),
            (
                "https://www.youtube.com/feeds/videos.xml?channel_id=UCFjOi1ZpZVErr8EYxg8t1dQ",
                "Dahboo",
            ),
            (
                "http://feeds.arstechnica.com/arstechnica/index",
                "arstechnica",
            ),
            (
                "https://www.gorillavsbear.net/category/mp3/feed/",
                "gorilla",
            ),
            (
                "http://feeds.feedburner.com/blogspot/cwWR",
                "financearmageddon",
            ),
            (
                "http://thesuperest.com/feed/rss.xml",
                "superest liste damaged",
            ),
            (
                "http://chaosradio.ccc.de/chaosradio-complete.rss",
                "chaosradio",
            ),
            (
                "http://feeds.seoulnews.net/rss/3f5c98640a497b43",
                "seoulnews - 기사 요약 -",
            ),
        ];

        let folder4 = feedsources.add_new_folder_at_parent("folder4".to_string(), 0);
        src.iter().for_each(|(url, desc)| {
            feedsources.add_new_feedsource_at_parent(
                url.to_string(),
                desc.to_string(),
                folder4,
                false,
            );
        });
    }
    // feedsources.set_fs_delete_id(Some(13));
    // feedsources.feedsource_delete();
}

// ------------------------------------
#[allow(unused_imports)]
#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = testing::logger_config::setup_logger();
    });
}