use fr_core::controller::contentdownloader::IDownloader;
use resources::parameter::DOWNLOADER_MAX_NUM_THREADS;

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
    fn is_dl_busy(&self) -> [u8; DOWNLOADER_MAX_NUM_THREADS] {
        [0; DOWNLOADER_MAX_NUM_THREADS]
    }
    fn cleanup_db(&self) {
        unimplemented!()
    }
    fn get_queue_size(&self) -> usize {
        unimplemented!()
    }
}
