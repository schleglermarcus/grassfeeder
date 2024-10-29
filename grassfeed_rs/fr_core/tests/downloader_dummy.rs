use fr_core::controller::contentdownloader::IDownloader;
use fr_core::controller::contentdownloader::DLKIND_MAX;

#[derive(Default)]
pub struct DownloaderDummy {}

impl DownloaderDummy {}

impl IDownloader for DownloaderDummy {
    fn add_update_subscription(&self, _f_source_repo_id: isize) {
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
    fn get_kind_list(&self) -> Vec<u8> {
        unimplemented!()
    }

    fn cleanup_db(&self) {
        unimplemented!()
    }
    fn get_queue_size(&self) -> (u16, u16) {
        unimplemented!()
    }

    fn browser_drag_request(&self, _dragged_url: &str) {
        unimplemented!()
    }

    fn launch_webbrowser(&self, _url: String, _cl_id: isize, _list_pos: u32) {
        unimplemented!()
    }

    fn get_statistics(&self) -> [u32; DLKIND_MAX] {
        unimplemented!()
    }
}
