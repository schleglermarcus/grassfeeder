use crate::db::subscription_entry::SubscriptionEntry;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::util::remove_invalid_chars_from_input;
use crate::util::timestamp_now;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use opml::Head;
use opml::Outline;
use opml::OPML;
use resources::gen_icons;
use std::cell::RefCell;
use std::fs::File;
use std::io::ErrorKind;
use std::rc::Rc;

const EXPORT_TITLE: &str = "GrassFeeder Export";

pub struct OpmlReader {
    root_outline: Outline,
    subscription_repo: Rc<RefCell<dyn ISubscriptionRepo>>,
}

impl OpmlReader {
    pub fn new(fsr_: Rc<RefCell<dyn ISubscriptionRepo>>) -> Self {
        OpmlReader {
            root_outline: Outline::default(),
            subscription_repo: fsr_,
        }
    }

    pub fn read_from_file(&mut self, filename: String) -> Result<(), Box<dyn std::error::Error>> {
        let r = std::fs::read_to_string(filename.clone());
        if r.is_err() {
            let current_dir = std::env::current_dir();
            error!(
                "reading  current_dir={:?}   {} : {:?}",
                current_dir, filename, &r
            );
            return Err(Box::new(r.err().unwrap()));
        }
        let opmlr = OPML::from_str(&r.unwrap());
        if opmlr.is_err() {
            error!("parsing opml  {} : {:?}", filename, &opmlr);
            return Err(Box::new(std::io::Error::new(
                ErrorKind::InvalidData,
                opmlr.err().unwrap(),
            )));
        }
        let o = opmlr.unwrap();
        self.root_outline = Outline::default();
        self.root_outline.version = Some(o.version.clone());
        if let Some(ref head) = o.head {
            self.root_outline.title = head.title.clone();
        }
        self.root_outline.outlines = o.body.outlines;
        Ok(())
    }

    pub fn transfer_to_db(&self, parent_folder_id: isize) {
        self.root_outline
            .outlines
            .iter()
            .enumerate()
            .for_each(|(i, o)| {
                OpmlReader::store_outline(
                    self.subscription_repo.clone(),
                    parent_folder_id,
                    i as isize,
                    o,
                );
            });
    }

    /// outline, parent_repo_id, position
    pub fn store_outline(
        subscription_repo: Rc<RefCell<dyn ISubscriptionRepo>>,
        parent_subs_id: isize,
        position: isize,
        outl: &Outline,
    ) {
        let db_entry = from_outline(outl, parent_subs_id, position);

        let repo_id: isize = match (*subscription_repo).borrow().store_entry(&db_entry) {
            Ok(r_entry) => {
                r_entry.subs_id
            }
            Err(e) => {
                error!("store_outline {:?}", &e);
                return;
            }
        };
        outl.outlines.iter().enumerate().for_each(|(folderpos, o)| {
            OpmlReader::store_outline(subscription_repo.clone(), repo_id, folderpos as isize, o);
        });
    }

    pub fn transfer_from_db(&mut self) {
        let fse = SubscriptionEntry {
            subs_id: 0,
            ..Default::default()
        };
        self.root_outline.title = Some(EXPORT_TITLE.to_string());
        OpmlReader::feedsource_to_outline(
            &mut self.root_outline,
            &fse,
            self.subscription_repo.clone(),
        );
    }
    /// recursive
    pub fn feedsource_to_outline(
        outline: &mut Outline,
        fse: &SubscriptionEntry,
        feed_src_repo: Rc<RefCell<dyn ISubscriptionRepo>>,
    ) {
        // debug!(            "feedsource_to_outline: {} {}",            &fse.repo_id, &fse.display_name        );
        let fse_list = (*feed_src_repo).borrow().get_by_parent_repo_id(fse.subs_id);
        for fse in fse_list {
            let mut outl = from_feed_source(&fse);
            if fse.is_folder {
                OpmlReader::feedsource_to_outline(&mut outl, &fse, feed_src_repo.clone());
            }
            (*outline).outlines.push(outl);
        }
    }

    pub fn write_to_file(&mut self, filename: String) -> Result<(), Box<dyn std::error::Error>> {
        let mut opml = OPML::default();
        opml.body.outlines = self.root_outline.outlines.clone();
        if let Some(ver) = self.root_outline.version.clone() {
            opml.version = ver;
        }
        if let Some(title) = self.root_outline.title.clone() {
            if opml.head.is_none() {
                opml.head = Some(Head::default());
            }
            if let Some(ref mut o_head) = opml.head {
                o_head.title = Some(title);
            }
        }
        let mut file = File::create(filename)?;
        opml.to_writer(&mut file)?;
        Ok(())
    }
}

impl Buildable for OpmlReader {
    type Output = OpmlReader;
    fn build(_conf: Box<dyn BuildConfig>, appcontext: &AppContext) -> Self::Output {
        let fsrr = appcontext.get_rc::<SubscriptionRepo>().unwrap();
        OpmlReader {
            root_outline: Outline::default(),
            subscription_repo: fsrr,
        }
    }
}

pub fn from_outline(o: &Outline, repo_parent_id: isize, folder_pos: isize) -> SubscriptionEntry {
    let mut is_folderr: bool = false;
    if let Some(ref outl_type) = o.r#type {
        if outl_type == "folder" {
            is_folderr = true;
        }
    }
    if !o.outlines.is_empty() {
        is_folderr = true;
    }
    let f_icon_id = if is_folderr {
        gen_icons::IDX_08_GNOME_FOLDER_48
    } else {
        gen_icons::IDX_05_RSS_FEEDS_GREY_64_D
    };

    let mut feed_url = String::default();
    let mut websit_url = String::default();
    if let Some(x_u) = &o.xml_url {
        feed_url = x_u.clone();
    }

    //TODO see if we can get the web main url from  the outline
    if let Some(h_u) = &o.html_url {
        if feed_url.is_empty() {
            feed_url = h_u.clone();
        } else {
            websit_url = h_u.clone();
        }
    }
    let displayname = remove_invalid_chars_from_input(o.text.clone());
    // trace!(        "from_outline:   text={}  #outl={} feed_url={}   website_url={}  folder={}",        &displayname,        o.outlines.len(),        &feed_url,        &websit_url,        is_folderr    );
    SubscriptionEntry {
        subs_id: 0,
        display_name: displayname,
        is_folder: is_folderr,
        url: feed_url,
        icon_id: f_icon_id,
        parent_subs_id: repo_parent_id,
        folder_position: folder_pos,
        updated_ext: 0,
        updated_int: timestamp_now(),
        updated_icon: 0,
        expanded: false,
        website_url: websit_url,
        last_selected_msg: -1,
        // num_msg_all_unread: None,
        // is_dirty: true,
        // status: 0,
        // tree_path: None,
        deleted: false,
    }
}

// o: &Outline, repo_parent_id: isize, folder_pos: usize
pub fn from_feed_source(fse: &SubscriptionEntry) -> Outline {
    Outline {
        r#type: if fse.is_folder {
            Some("folder".to_string())
        } else {
            Some("rss".to_string())
        },
        xml_url: Some(fse.url.clone()),
        text: fse.display_name.clone(),
        title: Some(fse.display_name.clone()),
        ..Default::default()
    }
}

impl StartupWithAppContext for OpmlReader {}

// ------------------------------------

#[cfg(test)]
mod t_ {
    use super::*;

    //RUST_BACKTRACE=1 cargo watch -s "cargo test  opml::opmlreader::t_::opml_import_w_folders  --lib -- --exact --nocapture"
    #[test]
    fn opml_import_w_folders() {
        setup();
        let fsrr = Rc::new(RefCell::new(SubscriptionRepo::new_inmem()));
		(*fsrr).borrow().scrub_all_subscriptions();
        let mut opmlreader = OpmlReader::new(fsrr.clone());
        let r = opmlreader.read_from_file(String::from("../testing/tests/opml/reader_wp.opml"));
        assert!(r.is_ok());
        opmlreader.transfer_to_db(0);
        let all = (*fsrr).borrow().get_all_entries();
        println!("all={:?}", all);
        let e0 = all.get(0).unwrap();
        assert!(e0.is_folder);
        assert_eq!(all.len(), 24);
    }

    //RUST_BACKTRACE=1 cargo watch -s "cargo test  opml::opmlreader::t_::opmlread_simple1  --lib -- --exact --nocapture"
    // #[ignore]
    #[test]
    fn opmlread_simple1() {
        let fsr = SubscriptionRepo::new_inmem();
		fsr.scrub_all_subscriptions();
        let fsrr = Rc::new(RefCell::new(fsr));
        let mut opmlreader = OpmlReader::new(fsrr.clone());
        let r = opmlreader.read_from_file(String::from("tests/data/simple_local.opml"));
        assert!(r.is_ok());
        opmlreader.transfer_to_db(0);
        let all = (*fsrr).borrow().get_all_entries();
        println!("ALL={:?}", all);
        assert_eq!(all.len(), 6);

        let e0 = all.get(0).unwrap();
        println!("0: {:?}", e0);
        assert_eq!(e0.is_folder, true);
        let e1 = all.get(1).unwrap();
        println!("1: {:?}", e1);
        assert_eq!(e1.parent_subs_id, e0.subs_id);
        assert_eq!(e1.is_folder, true);
        let e2 = all.get(2).unwrap();
        trace!("2: {:?}", e2.display_name);
        assert_eq!(e2.parent_subs_id, e1.subs_id);
        assert_eq!(e2.is_folder, false);
    }

    // #[ignore]
    #[test]
    fn opml_write() {
        let fsr = SubscriptionRepo::new_inmem();
		fsr.scrub_all_subscriptions();
        let fsrr: Rc<RefCell<dyn ISubscriptionRepo>> = Rc::new(RefCell::new(fsr));
        {
            let mut opmlreader = OpmlReader::new(fsrr.clone());
            let _r = opmlreader.read_from_file(String::from("tests/data/simple_local.opml"));
            opmlreader.transfer_to_db(0);
        }
        let dest_filename = "../target/written_simple.opml";
        let mut opmlreader = OpmlReader::new(fsrr.clone());
        opmlreader.transfer_from_db();
        let _r = opmlreader.write_to_file(String::from(dest_filename));
        let written_lenght = std::fs::metadata(dest_filename).unwrap().len();
        assert_eq!(written_lenght, 602);
    }

    #[allow(dead_code)]
    fn setup() {}
}
