use chrono::Local;
use chrono::TimeZone;
use context::appcontext::AppContext;
use fr_core::config::init_system::GrassFeederConfig;
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

// cargo watch -s "cargo run  --example working --features ui-gtk   "
fn main() {
    setup();
    loc::init_locales();
    let env_dir = std::env::var("PWD").unwrap();
    let dynamic_filename = format!("{}/target/dynamic.rss", env_dir);
    let mut mini_server_c = startup_minihttpserver(MINIHTTPSERVER_PORT);
    let _dyn_wr_handle = std::thread::spawn(move || loop {
        write_feed(&dynamic_filename);
        std::thread::sleep(std::time::Duration::from_secs(19));
    });
    let gfconf = GrassFeederConfig {
        path_config: format!("{}/target/db_rungui_local/", env_dir),
        path_cache: format!("{}/target/db_rungui_local/", env_dir),
        debug_mode: true,
        version: "rungui:rungui_local_clear".to_string(),
    };
    let appcontext = fr_core::config::init_system::start(gfconf);
    test_setup_values(&appcontext, mini_server_c.get_address());
    fr_core::config::init_system::run(&appcontext);
    mini_server_c.stop();
}

fn startup_minihttpserver(port: usize) -> MiniHttpServerController {
    let conf = ServerConfig {
        htdocs_dir: String::from("testing/tests/fr_htdocs"),
        index_file: String::from("index.html"),
        tcp_address: format!("127.0.0.1:{}", port).to_string(),
        binary_max_size: 1000000,
        download_throttling_kbps: 20,
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

fn write_feed(filename: &String) {
    setup();
    let header = "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>
<rss version=\"2.0\">
 <channel>
  <title>Dynamically created!</title>
  <description>some dynamic description:   lorem ipsum</description> \n";
    let footer = "\n </channel>\n</rss> \n";
    let ts_now = Local::now().timestamp();
    let o_file = File::create(filename);
    if o_file.is_err() {
        error!("cannot open {}", filename);
        return;
    }
    let mut file = o_file.unwrap();
    file.write(header.as_bytes()).unwrap();
    let entryline = entry(
        format!("TITLE-{}", ts_now).as_str(),
        "link",
        "description",
        ts_now,
    );
    file.write(entryline.as_bytes()).unwrap();
    let el2 = entry("statictitle", "link", "description", ts_now);
    file.write(el2.as_bytes()).unwrap();
    file.write(footer.as_bytes()).unwrap();
    // debug!("written to {} {}", RSS_DYNAMIC_FILENAME, ts_now);
}

fn test_setup_values(acr: &AppContext, addr: String) {
    if false {
        let messagesrepo_r: Rc<RefCell<dyn IMessagesRepo>> = acr.get_rc::<MessagesRepo>().unwrap();
        let _r = (*messagesrepo_r.borrow()).get_ctx().delete_table();
        (*messagesrepo_r.borrow()).get_ctx().create_table();
    }
    if true {
        let fsrwr: Rc<RefCell<dyn ISubscriptionRepo>> = acr.get_rc::<SubscriptionRepo>().unwrap();
        (*fsrwr.borrow()).scrub_all_subscriptions();
    }
    let feedsources_r: Rc<RefCell<dyn ISourceTreeController>> =
        acr.get_rc::<SourceTreeController>().unwrap();
    let ref mut feedsources = (*feedsources_r).borrow_mut();
    let url_dynamic = format!("{}/dynamic.rss", addr);
    let url_gui_proc = format!("{}/gui_proc_3.rss", addr);
    let url_feedburner = format!("{}/feedburner.rss", addr);
    let url_staseve = format!("{}/staseve-11.xml", addr);
    let url_r_foto = format!("{}/reddit-Fotografie.rss", addr);
    let url_insi = format!("{}/newsinsideout_com.rss", addr);
    let url_nn_aug = format!("{}/naturalnews_aug.xml", addr);
    // let url_relay = format!("{}/relay_rd.rss", addr); // very big

    let folder1 = feedsources.add_new_folder_at_parent("folder1".to_string(), 0);
    let folder2 = feedsources.add_new_folder_at_parent("folder2".to_string(), 0);
    let folder3 = feedsources.add_new_folder_at_parent("folder3".to_string(), folder1);
    let folder4 = feedsources.add_new_folder_at_parent("folder4".to_string(), folder1);

    // feedsources.add_new_subscription_at_parent(url_relay, "relay_rd too big".to_string(), folder2, false);
    feedsources.add_new_subscription_at_parent(url_dynamic, "dynamic".to_string(), folder1, false);

    if true {
        let src = [
            (url_feedburner.as_str(), "feedburner"),
            (url_insi.as_str(), "newsinsideout_com"),
            (url_r_foto.as_str(), "fotograf"),
        ];
        src.iter().for_each(|(url, desc)| {
            feedsources.add_new_subscription_at_parent(
                url.to_string(),
                desc.to_string(),
                folder2,
                false,
            );
        });
    }
    if false {
        feedsources.add_new_subscription_at_parent(
            url_nn_aug,
            "NN-aug".to_string(),
            folder1,
            false,
        );
        feedsources.add_new_subscription_at_parent(
            url_gui_proc.clone(),
            "gui_proc_2 & big-icon".to_string(),
            folder1,
            false,
        );
        feedsources.add_new_subscription_at_parent(
            url_staseve,
            "staseve11".to_string(),
            folder1,
            false,
        );
        let src = [
            ("https://m4rw3r.github.io/atom.xml", "marwer no icon"),
            ("https://www.fromrome.info/feed/", "fromrome icon okl"),
            ("https://www.relay.fm/query/feed", "relay_query icon ok"),
            ("https://www.mtb-karlsruhe.de/?q=rss.xml", "mb_ka icon ok"),
            ("http://n8waechter.info/feed/", "n8waechter  no-icon "),
            ("https://opposition24.com/feed/", "opposition"),
            ("http://lisahaven.news/feed/", "lisa_haven"), // original icon too big, scaled down.
            ("http://rss.slashdot.org/Slashdot/slashdot", "slashdot"), // sometimes delivers 403
            ("http://henrymakow.com/index.xml", "makow"),
            ("http://www.peopleofwalmart.com/feed/", "walmart-500"), // why error ?
            ("http://thehighersidechats.com/feed/", "higherside-300"),
            ("https://www.naturalnews.com/rss.xml", "naturalnews.com"),
            ("https://feeds.megaphone.fm/stuffyoushouldknow", "megaphone"),
            ("https://www.gistpaper.com/feed", "gistpaper"),
            ("http://xbustyx.xxxlog.co/feed/", "xbust_browser_hangs"),
            ("https://www.ksta.de/feed/index.rss", "Kö & ßtüdtänzêiger"),
            ("https://www.blacklistednews.com/rss.php", "blacklisted"), // hour-minute-seconds are all set to 0
            ("https://xkcd.com/atom.xml", "Xkcd-no-pubdate"),
            ("http://feeds.bbci.co.uk/news/rss.xml", "bbc"),
            ("https://www.headlinesoftoday.com/feed", "headlinesof"),
            ("https://insidexpress.com/feed/", "insidexpress"),
            ("https://linuxnews.de/feed/", "linuxnews"),
            ("https://linuxnews.de/comments/feed/", "linuxnews-comm"),
            ("https://www.linuxtoday.com/feed/", "linuxtoday"),
            ("https://news.itsfoss.com/feed/", "itsfoss"),
            ("https://www.openpr.de/rss/openpr.xml", "openpr"),
            ("https://www.reddit.com/r/funny.rss", "reddit-funny"),
            ("https://www.reddit.com/r/gaming.rss", "reddit-gaming"),
            ("https://tickernews.co/feed/", "tickernews"),
            ("https://tickernews.co/comments/feed/", "tickernews-comm"),
            ("https://www.gawker.com/rss", "gawker"),
            ("https://arcadi-online.de/feed/", "arcadi"),
            ("https://www.kino.de/rss/neu-im-kino", "kino_neu"),
            ("https://kolkatatv.org/feed/", "kolkatatv"),
            ("https://lupocattivoblog.com/feed/", "lupocat"),
            ("http://www.wissensmanufaktur.net/rss.xml", "wissensm"),
            ("http://feeds.feedburner.com/blogspot/cwWR", "financearmag"),
            ("http://newsinsideout.com/feed/", "newsinsideout"),
            ("https://sciencefiles.org/feed/", "science"),
            ("http://www.watergate.tv/feed/", "watergate"),
            ("http://www.guidograndt.de/feed/", "guido"),
            ("https://readrust.net/all/feed.rss", "readrust"),
            ("https://www.relay.fm/rd/feed", "rel_rd"),
            ("http://feeds.feedburner.com/euronews/en/news/", "euronews"),
            ("https://kodansha.us/feed/", "Kodansha"),
            ("https://planet.debian.org/rss20.xml", "debian"),
            ("https://report24.news/feed/", "report24"),
            ("https://www.heise.de/rss/heise-atom.xml", "heise-atom"),
            ("https://de.rt.com/feeds/news/", "RT DE"),
            ("https://terraherz.wpcomstaging.com/feed/", "terraherz"),
            ("https://www.reddit.com/r/aww.rss", "aww"),
            ("https://feeds.breakingnews.ie/bnworld", "breaknew"),
        ];
        src.iter().for_each(|(url, desc)| {
            feedsources.add_new_subscription_at_parent(
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
                "https://exopolitics.blogs.com/newsinsideout/atom.xml",
                "exopoli no icon",
            ),
            (
                "http://chaosradio.ccc.de/chaosradio-complete.rss",
                "chaosradio-267 icon ok",
            ),
            (
                "http://www.nachdenkseiten.de/?feed=atom",
                "nachdenk icon ok",
            ),
            (
                "http://www.ka-news.de/storage/rss/rss/karlsruhe.xml",
                "ka-news icon ok",
            ),
            (
                "https://www.asue.de/rss/gesamt.xml",
                "asue-no-pubdate icon ok  ",
            ),
            (
                "https://www.ft.com/news-feed?format=rss",
                "financialtimes icon ok",
            ),
            (
                "https://www.neweurope.eu/category/world/feed/",
                "neweurope icon ok",
            ),
            (
                "http://feeds.feedburner.com/TechmemeRideHome",
                "techmeme big-icon 4,7MB  !! ",
            ),
            (
                "https://www.buzzfeed.com/world.xml",
                "buzzfeed unknown icon",
            ),
            (
                "https://www.opendesktop.org/content.rdf",
                "opendesktop big-icon",
            ),
            (
                "http://www.channelnewsasia.com/rssfeeds/8395884",
                "newsasia big-icon",
            ),
            (
                "http://www.tagesschau.de/newsticker.rdf",
                "tagesschau-no-pubdate  big-icon",
            ),
            (
                "http://feeds.arstechnica.com/arstechnica/index",
                "arstechnica big-icon",
            ),
            (
                "https://www.gorillavsbear.net/category/mp3/feed/",
                "gorilla-mp3 big-icon",
            ),
            (
                "https://observer.ug/headlinenews?format=feed&type=rss",
                "obs_uganda ",
            ),
            (
                "https://nicheaddictgeneral.com/blogs/akah-ra.atom",
                "nicheaddict no-icon",
            ),
            (
                "https://www.linuxcompatible.org/news/atom.xml",
                "linuxcompatible",
            ),
            (
                "https://afternarcissisticabuse.wordpress.com/feed/",
                "afternarc",
            ),
            (
                "http://feeds.feedburner.com/RichardHerringLSTPodcast",
                "RichardHerring-560",
            ),
            (
                "https://www.youtube.com/feeds/videos.xml?channel_id=UC7nMSUJjOr7_TEo95Koudbg",
                "youtube",
            ),
            (
                "https://www.linuxcompatible.org/news/atom.xml",
                "linuxcompatible",
            ),
            (
                "https://packages.gentoo.org/packages/added.atom",
                "gentoo-added_no-pubdate-500",
            ), //  pubDate not there, but <updaed>
            (
                "https://www.thenexthint.com/feed/",
                "nexthint 無料ダウンロード",
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
                "https://www.euronews.com/rss?level=theme&name=news",
                "euronews",
            ),
            (
                "http://www.marketwatch.com/rss/realtimeheadlines",
                "marketwatch",
            ),
            (
                "http://rss.cnn.com/rss/edition_entertainment.rs",
                "cnn_entertain",
            ),
            (
                "https://www.youtube.com/feeds/videos.xml?channel_id=UCFjOi1ZpZVErr8EYxg8t1dQ",
                "Dahboo",
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
                "http://feeds.seoulnews.net/rss/3f5c98640a497b43",
                "seoulnews - 기사 요약 -",
            ),
        ];
        src.iter().for_each(|(url, desc)| {
            feedsources.add_new_subscription_at_parent(
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
        let _r = testing::logger_config_local::setup_logger();
    });
}
