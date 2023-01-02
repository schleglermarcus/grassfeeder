use crate::controller::browserpane::IBrowserPane;
use crate::controller::contentdownloader::IDownloader;
use crate::controller::contentlist::IFeedContents;
use crate::controller::guiprocessor::dl_char_for_kind;
use crate::controller::isourcetree::ISourceTreeController;
use crate::db::subscription_state::SubsMapEntry;
use crate::util::string_escape_url;
use crate::util::timestamp_now;
use gui_layer::abstract_ui::UIAdapterValueStoreType;
use gui_layer::abstract_ui::UIUpdaterAdapter;
use resources::id::LABEL_STATUS_1;
use resources::id::LABEL_STATUS_2;
use resources::id::LABEL_STATUS_3;
use resources::parameter::DOWNLOADER_MAX_NUM_THREADS;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

const BOTTOM_MSG_SHOW_TIME_S: u8 = 10;

// https://www.w3schools.com/charsets/ref_utf_block.asp
const VERTICAL_RISING_BAR: [u32; 9] = [
    ' ' as u32, 0x2581, 0x2582, 0x2583, 0x2584, 0x2585, 0x2586, 0x2587, 0x2588,
];

const VERTICAL_RISING_BAR_LEN: usize = VERTICAL_RISING_BAR.len() - 1;

// #[derive(Default)]
pub struct StatusBar {
    r_subscriptions_controller: Rc<RefCell<dyn ISourceTreeController>>,
    r_downloader: Rc<RefCell<dyn IDownloader>>,
    r_messages: Rc<RefCell<dyn IFeedContents>>,
    r_browserpane: Rc<RefCell<dyn IBrowserPane>>,
    gui_updater: Rc<RefCell<dyn UIUpdaterAdapter>>,
    gui_val_store: UIAdapterValueStoreType,
    downloader_kind: [u8; DOWNLOADER_MAX_NUM_THREADS],
    pub downloader_kind_new: [u8; DOWNLOADER_MAX_NUM_THREADS],
    num_msg_all: isize,
    num_msg_unread: isize,
    last_fetch_time: i64,
    selected_repo_id: isize,
    pub num_downloader_threads: u8,
    num_dl_queue_length: usize,
    selected_msg_id: i32,
    selected_msg_url: String,
    /// proc//status/VmRSS  Resident set size, estimation of the current physical memory used by the application
    pub mem_usage_vmrss_bytes: isize,
    pub mode_debug: bool,
    pub bottom_notices: VecDeque<String>,
    //  start-of-display  time,  current message
    bottom_notice_current: Option<(i64, String)>,
    pub browser_loading_progress: u8,
    browser_loading_progress_int: u8,
}

impl StatusBar {
    pub fn new(
        r_c_subs: Rc<RefCell<dyn ISourceTreeController>>,
        r_downloadr: Rc<RefCell<dyn IDownloader>>,
        gui_updatr: Rc<RefCell<dyn UIUpdaterAdapter>>,
        r_msg_c: Rc<RefCell<dyn IFeedContents>>,
        browser_pane: Rc<RefCell<dyn IBrowserPane>>,
        val_store: UIAdapterValueStoreType,
    ) -> Self {
        StatusBar {
            r_subscriptions_controller: r_c_subs,
            r_downloader: r_downloadr,
            gui_updater: gui_updatr,
            r_messages: r_msg_c,
            r_browserpane: browser_pane,
            gui_val_store: val_store,

            downloader_kind: Default::default(),
            downloader_kind_new: Default::default(),
            num_msg_all: Default::default(),
            num_msg_unread: Default::default(),
            last_fetch_time: Default::default(),
            selected_repo_id: Default::default(),
            num_downloader_threads: Default::default(),
            num_dl_queue_length: Default::default(),
            selected_msg_id: Default::default(),
            selected_msg_url: Default::default(),
            mem_usage_vmrss_bytes: Default::default(),
            mode_debug: false,
            bottom_notices: Default::default(),
            bottom_notice_current: Default::default(),
            browser_loading_progress: Default::default(),
            browser_loading_progress_int: Default::default(),
        }
    }

    // #[allow(clippy::needless_range_loop)] // handle this later
    pub fn update(&mut self) {
        let mut need_update1: bool = false;
        let mut need_update2: bool = false;
        let mut need_update3: bool = false;
        let repo_id_new: isize;
        let mut last_fetch_time: i64 = 0;
        let mut feed_src_link = String::default();
        let mut is_folder: bool = false;
        let o_fse = (*self.r_subscriptions_controller)
            .borrow()
            .get_current_selected_subscription();
        if let Some((fse, _)) = o_fse {
            repo_id_new = fse.subs_id;
            last_fetch_time = fse.updated_int;
            feed_src_link = fse.url.clone();
            self.num_downloader_threads = (*self.r_downloader)
                .borrow()
                .get_config()
                .num_downloader_threads;
            is_folder = fse.is_folder;
        } else {
            repo_id_new = -1;
        }
        let content_ids = (*self.r_messages).borrow().get_selected_content_ids();
        let mut selected_msg_id = -1;
        if !content_ids.is_empty() {
            selected_msg_id = *content_ids.first().unwrap();
        }
        let mut num_msg_all = self.num_msg_all;
        let mut num_msg_unread = self.num_msg_unread;
        let subs_state: SubsMapEntry = (*self.r_subscriptions_controller)
            .borrow()
            .get_state(repo_id_new)
            .unwrap_or_default();
        if selected_msg_id != self.selected_msg_id || repo_id_new != self.selected_repo_id {
            self.selected_msg_id = selected_msg_id;
        }
        if let Some((n_a, n_u)) = subs_state.num_msg_all_unread {
            num_msg_all = n_a;
            num_msg_unread = n_u;
            if n_a != self.num_msg_all || n_u != self.num_msg_unread {
                need_update2 = true;
            }
        }
        if repo_id_new > 0 {
            if num_msg_all != self.num_msg_all {
                self.num_msg_all = num_msg_all;
                need_update1 = true;
            }
            if num_msg_unread != self.num_msg_unread {
                self.num_msg_unread = num_msg_unread;
                need_update1 = true;
            }

            if self.last_fetch_time != last_fetch_time {
                self.last_fetch_time = last_fetch_time;
                need_update2 = true;
            }
        }
        let last_msg_url = if selected_msg_id < 0 {
            String::default()
        } else {
            (self.r_browserpane).borrow().get_last_selected_link()
        };
        if self.selected_msg_url != last_msg_url {
            self.selected_msg_url = last_msg_url;
            need_update2 = true;
        }
        let timestamp_now: i64 = timestamp_now();
        let mut longtext = if self.selected_msg_url.is_empty() {
            string_escape_url(feed_src_link)
        } else {
            string_escape_url(self.selected_msg_url.clone())
        };
        if let Some((ts, msg)) = &self.bottom_notice_current {
            if timestamp_now > ts + BOTTOM_MSG_SHOW_TIME_S as i64 {
                self.bottom_notice_current = None;
            } else {
                longtext = StatusBar::error_formatter(msg.to_string());
            }
            need_update2 = true;
        } else if let Some(n_msg) = self.bottom_notices.pop_front() {
            self.bottom_notice_current = Some((timestamp_now, n_msg));
            need_update2 = true;
        }
        let mut block_vertical: char = ' ';
        if !is_folder && repo_id_new != self.selected_repo_id {
            self.selected_repo_id = repo_id_new;
            // time-to next feed update
            let fs_conf = self.r_subscriptions_controller.borrow().get_config();
            let interval_s = (*fs_conf).borrow().get_interval_seconds();
            let elapsed: i64 = std::cmp::min(timestamp_now - (last_fetch_time), interval_s);
            block_vertical = self.get_vertical_block_char(elapsed as usize, interval_s as usize);
            need_update2 = true;
            need_update1 = true;
        }
        let downloader_busy = (self.r_downloader).borrow().get_kind_list();
        for (a, busy) in downloader_busy
            .iter()
            .enumerate()
            .take(DOWNLOADER_MAX_NUM_THREADS)
        {
            if self.downloader_kind[a] > 0 && *busy == 0 {
                self.downloader_kind_new[a] = 0;
                need_update1 = true;
            }
            if self.downloader_kind[a] != self.downloader_kind_new[a] {
                self.downloader_kind[a] = self.downloader_kind_new[a];
                need_update1 = true;
            }
        }
        let new_qsize = (*self.r_downloader).borrow().get_queue_size();
        if new_qsize != self.num_dl_queue_length {
            self.num_dl_queue_length = new_qsize;
            need_update1 = true;
        }
        if self.browser_loading_progress != self.browser_loading_progress_int {
            self.browser_loading_progress_int = self.browser_loading_progress;
            need_update3 = true;
        }
        if need_update1 {
            let mut downloader_display: String = String::default();
            for a in 0..(self.num_downloader_threads as usize) {
                let nc = dl_char_for_kind(self.downloader_kind[a]);
                downloader_display.push(nc);
            }
            let unread_all = format!("{:5} / {:5}", self.num_msg_unread, self.num_msg_all);
            let memdisplay = if self.mode_debug {
                format!("  {}MB ", self.mem_usage_vmrss_bytes / 1048576,)
            } else {
                String::default()
            };
            let msg1 = format!(
                "<tt>\u{25df}{}\u{25de} {}    \u{2595}{}\u{258F} {}</tt>",
                downloader_display, unread_all, block_vertical, memdisplay,
            );
            (*self.gui_val_store)
                .write()
                .unwrap()
                .set_label_text(LABEL_STATUS_1, msg1);
            let msg1tooltip = if self.num_dl_queue_length > 0 {
                format!("Queue: {}", self.num_dl_queue_length)
            } else {
                String::default()
            };
            (*self.gui_val_store)
                .write()
                .unwrap()
                .set_label_tooltip(LABEL_STATUS_1, msg1tooltip);

            (*self.gui_updater)
                .borrow()
                .update_label_markup(LABEL_STATUS_1);
        }
        if need_update2 {
            (*self.gui_val_store)
                .write()
                .unwrap()
                .set_label_text(LABEL_STATUS_2, longtext);
            (*self.gui_updater)
                .borrow()
                .update_label_markup(LABEL_STATUS_2);
        }
        if need_update3 {
            let b_loading =
                self.get_vertical_block_char(self.browser_loading_progress as usize, 256);
            (*self.gui_val_store)
                .write()
                .unwrap()
                .set_label_text(LABEL_STATUS_3, format!("<tt>\u{2595}{}</tt>", b_loading));
            (*self.gui_updater)
                .borrow()
                .update_label_markup(LABEL_STATUS_3);
        }
    }

    // Mem usage in kb: current=105983, peak=118747411
    // Htop:  103M    SHR: 73580m   0,7% mem
    // top:	Res:106MB   SHR:78MB
    // estimation of the current physical memory used by the application, in bytes. 			Comes from proc//status/VmRSS
    pub fn update_memory_stats(&mut self) {
        if let Ok(mem) = proc_status::mem_usage() {
            self.mem_usage_vmrss_bytes = (self.mem_usage_vmrss_bytes + mem.current as isize) / 2;
        }
    }

    fn error_formatter(s: String) -> String {
        format!(
            "<span foreground=\"#CC6666\">{}</span>",
            string_escape_url(s)
        )
    }

    fn get_vertical_block_char(&self, dividend: usize, divisor: usize) -> char {
        let div_idx = if divisor == 0 {
            0
        } else {
            dividend * VERTICAL_RISING_BAR_LEN / divisor
        };
        char::from_u32(VERTICAL_RISING_BAR[div_idx]).unwrap()
    }
}
