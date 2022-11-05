use fr_core::config::configmanager::ConfigManager;
use fr_core::controller::contentdownloader::IDownloader;
use fr_core::controller::sourcetree::ISourceTreeController;
use fr_core::controller::sourcetree::SourceTreeController;
use fr_core::db::errors_repo::ErrorRepo;
use fr_core::db::icon_repo::IconRepo;
use fr_core::db::subscription_entry::SubscriptionEntry;
use fr_core::db::subscription_repo::ISubscriptionRepo;
use fr_core::db::subscription_repo::SubscriptionRepo;
use fr_core::controller::timer::build_timer;
use fr_core::controller::timer::ITimer;
use fr_core::ui_select::uimock::UIMock;
use std::cell::RefCell;
use std::rc::Rc;

#[allow(dead_code)]
pub fn prepare_source_tree_controller(
    fs_list: Vec<SubscriptionEntry>,
) -> (SourceTreeController, Rc<RefCell<dyn ISubscriptionRepo>>) {
    let subscrip_repo = SubscriptionRepo::new_inmem(); // new("");
    subscrip_repo.scrub_all_subscriptions();
    fs_list.iter().for_each(|e| {
        let _r = subscrip_repo.store_entry(e);
    });
    let r_subscriptions_repo: Rc<RefCell<dyn ISubscriptionRepo>> =
        Rc::new(RefCell::new(subscrip_repo));
    let r_icons_repo = Rc::new(RefCell::new(IconRepo::new("")));
    let r_timer: Rc<RefCell<dyn ITimer>> = Rc::new(RefCell::new(build_timer()));
    let uimock = UIMock::new();
    let downloaderdummy = DownloaderDummy::default();
    let r_dl: Rc<RefCell<dyn IDownloader>> = Rc::new(RefCell::new(downloaderdummy));
    let r_configmanager = Rc::new(RefCell::new(ConfigManager::default()));
    let r_error_repo = Rc::new(RefCell::new(ErrorRepo::new(&String::default())));
    let fs = SourceTreeController::new(
        r_timer,
        r_subscriptions_repo.clone(),
        r_configmanager,
        r_icons_repo,
        uimock.upd_adp(),
        uimock.val_sto(),
        r_dl,
        r_error_repo,
    );
    fs.update_cached_paths();
    (fs, r_subscriptions_repo)
}

/// prepares 3 sources in a row, no folders
#[allow(dead_code)]
pub fn dataset_simple_trio() -> Vec<SubscriptionEntry> {
    let mut fs_list: Vec<SubscriptionEntry> = Vec::default();
    let mut fse =
        SubscriptionEntry::from_new_url("feed1-display".to_string(), "feed1-url".to_string());
    fse.subs_id = 1;
    fse.folder_position = 0;
    fs_list.push(fse.clone());

    fse.display_name = "feed2-display".to_string();
    fse.url = "feed2-url".to_string();
    fse.subs_id = 2;
    fse.folder_position = 1;
    fs_list.push(fse.clone());

    fse.display_name = "feed3-display".to_string();
    fse.url = "feed3-url".to_string();
    fse.subs_id = 3;
    fse.folder_position = 2;
    fs_list.push(fse.clone());
    fs_list
}

#[allow(dead_code)]
pub fn dataset_three_folders() -> Vec<SubscriptionEntry> {
    let mut fs_list: Vec<SubscriptionEntry> = Vec::default();
    let mut fse = SubscriptionEntry::from_new_foldername("folder1".to_string(), 0);
    fse.subs_id = 1;
    fse.folder_position = 0;
    fs_list.push(fse.clone());

    fse.display_name = "folder2".to_string();
    fse.subs_id = 2;
    fse.folder_position = 1;
    fs_list.push(fse.clone());

    fse.display_name = "folder3".to_string();
    fse.subs_id = 3;
    fse.folder_position = 2;
    fs_list.push(fse.clone());
    fs_list
}

/*
+
  - folder1
    - feed2d
    - feed3d
  - folder4
    - feed5d
*/
#[allow(dead_code)]
pub fn dataset_some_tree() -> Vec<SubscriptionEntry> {
    let mut fs_list: Vec<SubscriptionEntry> = Vec::default();

    let mut fse = SubscriptionEntry::from_new_foldername("folder1".to_string(), 0);
    fse.subs_id = 1;
    fs_list.push(fse.clone());

    fse.subs_id = 2;
    fse.is_folder = false;
    fse.display_name = "feed2d".to_string();
    fse.folder_position = 0;
    fse.parent_subs_id = 1;
    fs_list.push(fse.clone());

    fse.subs_id = 3;
    fse.folder_position = 1;
    fse.parent_subs_id = 1;
    fse.is_folder = false;
    fse.display_name = "feed3d".to_string();
    fse.url = "feed4-url".to_string();
    fs_list.push(fse.clone());

    fse.subs_id = 4;
    fse.is_folder = true;
    fse.display_name = "folder4".to_string();
    fse.folder_position = 1;
    fse.parent_subs_id = 0;
    fs_list.push(fse.clone());

    fse.subs_id = 5;
    fse.folder_position = 0;
    fse.parent_subs_id = 4;
    fse.is_folder = false;
    fse.display_name = "feed5d".to_string();
    fs_list.push(fse.clone());

    //fs_list.iter().for_each(|fs| debug!("some_tree  {}", fs));
    fs_list
}

#[derive(Default)]
pub struct DownloaderDummy {}

impl DownloaderDummy {}

impl IDownloader for DownloaderDummy {
    fn add_update_source(&self, _f_source_repo_id: isize) {
        unimplemented!()
    }
    fn load_icon(&self, _fs_id: isize, _fs_url: String, _old_icon_id: usize) {
        unimplemented!()
    }
    fn new_feedsource_request(&self, _fs_edit_url: &str) {
        unimplemented!()
    }
    fn shutdown(&mut self) {
        unimplemented!()
    }
    fn is_running(&self) -> bool {
        unimplemented!()
    }
    fn get_config(&self) -> fr_core::controller::contentdownloader::Config {
        unimplemented!()
    }
    fn set_conf_num_threads(&mut self, _: u8) {
        unimplemented!()
    }
    fn cleanup_db(&self) {
        unimplemented!()
    }
    fn get_queue_size(&self) -> usize {
        unimplemented!()
    }

    fn get_kind_list(&self) -> Vec<u8> {
        unimplemented!()
    }

    // fn download_direct(&self, _url: &String) -> Result<String, String> {        unimplemented!()    }

    fn browser_drag_request(&self, _dragged_url: &str) {
        unimplemented!()
    }
}
