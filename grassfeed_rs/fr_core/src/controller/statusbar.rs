use crate::controller::browserpane::IBrowserPane;
use crate::controller::contentdownloader::IDownloader;
use crate::controller::contentdownloader::DLKIND_MAX;
use crate::controller::contentlist::IContentList;
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

trait OnePanel {
    /// returns: regular text, tooltip-text
    fn calculate_update(&self, statusbar: &StatusBar) -> (Option<String>, Option<String>);
    fn get_label_id(&self) -> u8;
}

// #[derive(Default)]
pub struct StatusBar {
    r_subscriptions_controller: Rc<RefCell<dyn ISourceTreeController>>,
    r_downloader: Rc<RefCell<dyn IDownloader>>,
    r_messages: Rc<RefCell<dyn IContentList>>,
    r_browserpane: Rc<RefCell<dyn IBrowserPane>>,
    gui_updater: Rc<RefCell<dyn UIUpdaterAdapter>>,
    gui_val_store: UIAdapterValueStoreType,
    pub cache: RefCell<CachedData>,
    panels: Vec<Box<dyn OnePanel>>,
}

impl StatusBar {
    pub fn new(
        r_c_subs: Rc<RefCell<dyn ISourceTreeController>>,
        r_downloadr: Rc<RefCell<dyn IDownloader>>,
        gui_updatr: Rc<RefCell<dyn UIUpdaterAdapter>>,
        r_msg_c: Rc<RefCell<dyn IContentList>>,
        browser_pane: Rc<RefCell<dyn IBrowserPane>>,
        val_store: UIAdapterValueStoreType,
    ) -> Self {
        let mut panels_: Vec<Box<dyn OnePanel>> = Vec::new();
        panels_.push(Box::new(PanelLeft {}));
        panels_.push(Box::new(PanelMiddle {}));
        panels_.push(Box::new(PanelRight {}));

        StatusBar {
            r_subscriptions_controller: r_c_subs,
            r_downloader: r_downloadr,
            gui_updater: gui_updatr,
            r_messages: r_msg_c,
            r_browserpane: browser_pane,
            gui_val_store: val_store,
            cache: RefCell::new(CachedData::default()),
            panels: panels_, //  Vec::default(),
        }
    }

    pub fn set_downloader_kind(&self, threadnr: u8, kind: u8) {
        self.cache.borrow_mut().downloader_kind_new[threadnr as usize] = kind;
    }

    pub fn set_db_check_running(&self, r: bool) {
        self.cache.borrow_mut().db_check_running = r;
    }

    pub fn is_db_check_running(&self) -> bool {
        self.cache.borrow().db_check_running
    }

    pub fn push_bottom_notice(&self, msg: String) {
        self.cache.borrow_mut().bottom_notices.push_back(msg);
    }

    pub fn get_bottom_notice_current(&self) -> Option<(i64, String)> {
        self.cache.borrow().bottom_notice_current.clone()
    }

    pub fn get_db_check_message(&self) -> String {
        self.cache.borrow().db_check_display_message.clone()
    }

    pub fn set_db_check_message(&self, newmsg: String) {
        self.cache.borrow_mut().db_check_display_message = newmsg;
    }

    pub fn set_mem_vrmss_bytes(&self, numbytes: isize) {
        self.cache.borrow_mut().mem_usage_vmrss_bytes = numbytes;
    }

    pub fn set_num_downloader_threads(&self, n: u8) {
        self.cache.borrow_mut().num_downloader_threads = n;
    }

    pub fn set_mode_debug(&self, debu: bool) {
        self.cache.borrow_mut().mode_debug = debu;
    }

    pub fn set_browser_loading_progress(&self, p: u8) {
        self.cache.borrow_mut().browser_loading_progress = p;
    }

    pub fn pop_bottom_message(&self) -> Option<String> {
        self.cache.borrow_mut().pop_bottom_message_int()
    }

    pub fn update(&self) {
        for p in &self.panels {
            let label_id = p.get_label_id();
            let (o_labeltext, o_tooltip) = p.calculate_update(&self);
            let do_update = o_tooltip.is_some() || o_labeltext.is_some();
            if let Some(labeltext) = o_labeltext {
                (*self.gui_val_store)
                    .write()
                    .unwrap()
                    .set_label_text(label_id, labeltext);
            }
            if let Some(tt) = o_tooltip {
                (*self.gui_val_store)
                    .write()
                    .unwrap()
                    .set_label_tooltip(label_id, tt);
            }
            if do_update {
                (*self.gui_updater).borrow().update_label_markup(label_id);
            }
        }

        self.update_old();
    }

    pub fn update_old(&self) {
        // let mut last_fetch_time: i64 = 0;
        let mut feed_src_link = String::default();
        //      let mut is_folder: bool = false;
        // let timestamp_now: i64 = timestamp_now();

        let mut subscription_id_new: isize = -1;
        let o_subscription = (*self.r_subscriptions_controller)
            .borrow()
            .get_current_selected_subscription();
        if let Some((fse, _)) = o_subscription {
            subscription_id_new = fse.subs_id;
            //             last_fetch_time = fse.updated_int;
            feed_src_link.clone_from(&fse.url);
            //            is_folder = fse.is_folder;
            let dl_r_b = (*self.r_downloader).borrow();
            let mut c = self.cache.borrow_mut();
            c.num_downloader_threads = dl_r_b.get_config().num_downloader_threads;
            c.downloader_stats = dl_r_b.get_statistics();
            c.subscription_is_folder = fse.is_folder;
            c.subscription_last_download_time = fse.updated_int;
        } else {
            subscription_id_new = -1;
        }

        let subscription_id_old = self.cache.borrow().selected_repo_id;
        if subscription_id_new != subscription_id_old {
            let mut c = self.cache.borrow_mut();
            c.selected_repo_id = subscription_id_new;
            c.subscription_id_changed = true;
        } else {
            self.cache.borrow_mut().subscription_id_changed = false;
        }

        let content_ids = (*self.r_messages).borrow().get_selected_content_ids();
        let mut selected_msg_id = -1;
        if !content_ids.is_empty() {
            selected_msg_id = *content_ids.first().unwrap();
        }
        if selected_msg_id != self.cache.borrow().selected_msg_id
            || subscription_id_new != self.cache.borrow().selected_repo_id
        {
            self.cache.borrow_mut().selected_msg_id = selected_msg_id;
        }

        /*
               // label-3
               {
                   let mut need_update3: bool = false;

                   let p_int = self.cache.borrow().browser_loading_progress_int;
                   if self.cache.borrow().browser_loading_progress != p_int {
                       self.cache.borrow_mut().browser_loading_progress_int = p_int;
                       need_update3 = true;
                   }
                   if need_update3 {
                       let b_loading = get_vertical_block_char(p_int as usize, 256);
                       (*self.gui_val_store)
                           .write()
                           .unwrap()
                           .set_label_text(LABEL_STATUS_3, format!("<tt>\u{2595}{b_loading}</tt>"));
                       (*self.gui_updater)
                           .borrow()
                           .update_label_markup(LABEL_STATUS_3);
                   }
               }
        */
    }

    // Mem usage in kb: current=105983, peak=118747411
    // Htop:  103M    SHR: 73580m   0,7% mem
    // top:	Res:106MB   SHR:78MB
    // estimation of the current physical memory used by the application, in bytes.
    // 		Comes from proc//status/VmRSS
    pub fn update_memory_stats(&self) {
        if let Ok(mem) = proc_status::mem_usage() {
            // trace!(                "PS:  MEM:  {} {}",                mem.current / DIVISOR_MB,                mem.peak / DIVISOR_MB            );
            let vmrss_bytes =
                (self.cache.borrow().mem_usage_vmrss_bytes + mem.current as isize) / 2;
            self.cache.borrow_mut().mem_usage_vmrss_bytes = vmrss_bytes;
        }
    }

    fn error_formatter(s: String) -> String {
        format!(
            "<span foreground=\"#CC6666\">{}</span>",
            string_escape_url(s)
        )
    }

    /// Text field inside the Settings Dialog, DB-Cleanup Tab
    pub fn set_db_check_msg(&mut self, m: &str) {
        m.clone_into(&mut self.cache.borrow_mut().db_check_display_message);
    }
}

fn get_vertical_block_char(dividend: usize, divisor: usize) -> char {
    let div_idx = if divisor == 0 {
        0
    } else {
        dividend * VERTICAL_RISING_BAR_LEN / divisor
    };
    char::from_u32(VERTICAL_RISING_BAR[div_idx]).unwrap()
}

#[derive(Default)]
pub struct CachedData {
    pub downloader_kind: [u8; DOWNLOADER_MAX_NUM_THREADS],
    pub downloader_kind_new: [u8; DOWNLOADER_MAX_NUM_THREADS],
    pub num_msg_all: isize,
    pub num_msg_unread: isize,
    pub last_fetch_time: i64,
    pub num_downloader_threads: u8,
    pub num_dl_queue_length: u16,
    pub selected_msg_id: i32,
    pub selected_msg_url: String,
    /// proc//status/VmRSS  Resident set size, estimation of the current physical memory used by the application
    pub mem_usage_vmrss_bytes: isize,
    pub mode_debug: bool,
    pub bottom_notices: VecDeque<String>,
    //  start-of-display  time,  current message
    pub bottom_notice_current: Option<(i64, String)>,
    pub browser_loading_progress: u8,
    pub browser_loading_progress_int: u8,
    pub downloader_stats: [u32; DLKIND_MAX],
    pub db_check_running: bool,
    pub db_check_display_message: String,

    ///  change name
    pub selected_repo_id: isize,
    pub subscription_id_changed: bool,

    pub subscription_is_folder: bool,
    pub subscription_last_download_time: i64,
}

impl CachedData {
    pub fn pop_bottom_message_int(&mut self) -> Option<String> {
        let o_m = self.bottom_notices.pop_front();
        if o_m.is_none() {
            self.bottom_notice_current = None;
            return None;
        }
        let msg = o_m.unwrap();
        self.bottom_notice_current = Some((timestamp_now(), msg.clone()));
        Some(msg)
    }
}

struct PanelLeft {}

impl OnePanel for PanelLeft {
    fn calculate_update(&self, statusbar: &StatusBar) -> (Option<String>, Option<String>) {
        // label-1
        let timestamp_now: i64 = timestamp_now();
        let mut need_update_1: bool = false;
        let mut num_msg_all: isize;
        num_msg_all = statusbar.cache.borrow().num_msg_all;
        let mut num_msg_unread = statusbar.cache.borrow().num_msg_unread;
        let mut is_folder: bool = false;

        let subscription_id = statusbar.cache.borrow().selected_repo_id;

        let subs_state: SubsMapEntry = (*statusbar.r_subscriptions_controller)
            .borrow()
            .get_state(subscription_id)
            .unwrap_or_default();

        if let Some((n_a, n_u)) = subs_state.num_msg_all_unread {
            num_msg_all = n_a;
            num_msg_unread = n_u;
        }

        if subscription_id > 0 {
            if num_msg_all != statusbar.cache.borrow().num_msg_all {
                statusbar.cache.borrow_mut().num_msg_all = num_msg_all;
                need_update_1 = true;
            }
            if num_msg_unread != statusbar.cache.borrow().num_msg_unread {
                statusbar.cache.borrow_mut().num_msg_unread = num_msg_unread;
                need_update_1 = true;
            }
        }
        let mut block_vertical: char = ' ';

        let subs_changed: bool;

        if !is_folder && statusbar.cache.borrow().subscription_id_changed {
            // statusbar.cache.borrow_mut().selected_repo_id = repo_id_new;
            let fs_conf = statusbar.r_subscriptions_controller.borrow().get_config();
            let last_fetch_time = statusbar.cache.borrow().subscription_last_download_time;
            let interval_s = (*fs_conf).borrow().get_interval_seconds();
            let elapsed: i64 = std::cmp::min(timestamp_now - (last_fetch_time), interval_s);
            block_vertical = get_vertical_block_char(elapsed as usize, interval_s as usize);
            need_update_1 = true;
        }

        let downloader_busy = (statusbar.r_downloader).borrow().get_kind_list();
        for (a, busy) in downloader_busy
            .iter()
            .enumerate()
            .take(DOWNLOADER_MAX_NUM_THREADS)
        {
            if statusbar.cache.borrow().downloader_kind[a] > 0 && *busy == 0 {
                statusbar.cache.borrow_mut().downloader_kind_new[a] = 0;
                need_update_1 = true;
            }

            let k_new = statusbar.cache.borrow().downloader_kind_new[a];
            if statusbar.cache.borrow().downloader_kind[a] != k_new {
                statusbar.cache.borrow_mut().downloader_kind[a] = k_new;
                need_update_1 = true;
            }
        }
        let new_qsize = (*statusbar.r_downloader).borrow().get_queue_size();
        let new_qsize = new_qsize.0 + new_qsize.1; // queue + threads
        if new_qsize != statusbar.cache.borrow().num_dl_queue_length {
            statusbar.cache.borrow_mut().num_dl_queue_length = new_qsize;
            need_update_1 = true;
        }

        if !need_update_1 {
            return (None, None);
        }

        let mut downloader_display: String = String::default();
        let n_threads: usize = statusbar.cache.borrow().num_downloader_threads as usize;
        for a in 0..n_threads {
            let nc = dl_char_for_kind(statusbar.cache.borrow().downloader_kind[a]);
            downloader_display.push(nc);
        }
        let unread_all = format!(
            "{:5} / {:5}",
            statusbar.cache.borrow().num_msg_unread,
            statusbar.cache.borrow().num_msg_all
        );
        let mut dl_line = String::default();
        statusbar
            .cache
            .borrow()
            .downloader_stats
            .iter()
            .enumerate()
            .filter(|(_n, s)| **s > 0)
            .map(|(n, s)| format!("{}{} ", dl_char_for_kind(n as u8), s))
            .for_each(|s| dl_line.push_str(&s));
        let memdisplay = format!(
            "  {}MB  {}  ",
            statusbar.cache.borrow().mem_usage_vmrss_bytes / 1048576,
            dl_line
        );
        let dl_queue_txt = if statusbar.cache.borrow().num_dl_queue_length > 0 {
            format!("{:2}", statusbar.cache.borrow().num_dl_queue_length)
        } else {
            "  ".to_string()
        };
        let msg1 = format!(   "<tt>{dl_queue_txt} {downloader_display}  {unread_all}    \u{2595}{block_vertical}\u{258F}</tt>"   );
        // debug!("PanelLeft:3    {msg1}  {memdisplay}  ");
        (Some(msg1), Some(memdisplay))
    }
    fn get_label_id(&self) -> u8 {
        LABEL_STATUS_1
    }
}

struct PanelMiddle {}
impl OnePanel for PanelMiddle {
    fn calculate_update(&self, statusbar: &StatusBar) -> (Option<String>, Option<String>) {
        let mut need_update_2: bool = false;

        let mut subscription_id_new: isize = statusbar.cache.borrow().selected_repo_id;

        let mut feed_src_link = String::default();
        let o_subscription = (*statusbar.r_subscriptions_controller)
            .borrow()
            .get_current_selected_subscription();
        if let Some((fse, _)) = o_subscription {
            subscription_id_new = fse.subs_id;
            feed_src_link.clone_from(&fse.url);
        } else {
            subscription_id_new = -1;
        }

        let subs_state: SubsMapEntry = (*statusbar.r_subscriptions_controller)
            .borrow()
            .get_state(subscription_id_new)
            .unwrap_or_default();

        if let Some((n_a, n_u)) = subs_state.num_msg_all_unread {
            if n_a != statusbar.cache.borrow().num_msg_all
                || n_u != statusbar.cache.borrow().num_msg_unread
            {
                need_update_2 = true;
            }
        }

        let content_ids = (*statusbar.r_messages).borrow().get_selected_content_ids();
        let mut selected_msg_id = -1;
        if !content_ids.is_empty() {
            selected_msg_id = *content_ids.first().unwrap();
        }
        if selected_msg_id != statusbar.cache.borrow().selected_msg_id
            || subscription_id_new != statusbar.cache.borrow().selected_repo_id
        {
            statusbar.cache.borrow_mut().selected_msg_id = selected_msg_id;
        }

        // if subscription_id_new > 0 {
        //     if self.cache.borrow().last_fetch_time != last_fetch_time {
        //         self.cache.borrow_mut().last_fetch_time = last_fetch_time;
        //         need_update_2 = true;
        //     }
        // }

        let last_msg_url = if selected_msg_id < 0 {
            String::default()
        } else {
            (statusbar.r_browserpane).borrow().get_last_selected_link()
        };
        if statusbar.cache.borrow().selected_msg_url != last_msg_url {
            statusbar.cache.borrow_mut().selected_msg_url = last_msg_url;
            need_update_2 = true;
        }

        let timestamp_now: i64 = timestamp_now();

        let selected_msg_url: String = statusbar.cache.borrow().selected_msg_url.clone();
        let mut longtext = if selected_msg_url.is_empty() {
            string_escape_url(feed_src_link)
        } else {
            string_escape_url(selected_msg_url.clone())
        };

        let o_current = &statusbar.get_bottom_notice_current();
        if let Some((ts, msg)) = o_current {
            if timestamp_now > ts + BOTTOM_MSG_SHOW_TIME_S as i64 {
                statusbar.cache.borrow_mut().bottom_notice_current = None;
            } else {
                longtext = StatusBar::error_formatter(msg.to_string());
            }
            need_update_2 = true;
        }
        //  TODO    bottom notice  needs rework
        if o_current.is_none() {
            if let Some(_n_msg) = statusbar.pop_bottom_message() {
                need_update_2 = true;
            }
        }

        // debug!("Middle: {}  {} ", need_update_2, longtext);
        if !statusbar.cache.borrow().subscription_is_folder
            && subscription_id_new != statusbar.cache.borrow().selected_repo_id
        {
            statusbar.cache.borrow_mut().selected_repo_id = subscription_id_new;
            need_update_2 = true;
        }

        if need_update_2 {
            return (Some(longtext), None);
        }

        (None, None)
    }

    fn get_label_id(&self) -> u8 {
        LABEL_STATUS_2
    }
}

struct PanelRight {}
impl OnePanel for PanelRight {
    fn calculate_update(&self, statusbar: &StatusBar) -> (Option<String>, Option<String>) {
        let progr = statusbar.cache.borrow().browser_loading_progress;
        let p_int = statusbar.cache.borrow().browser_loading_progress_int;
        if p_int > 0 || progr > 0 {
            debug!("right:  p: {}   <== {}   ", p_int, progr);
        }

        if progr != p_int {
            statusbar.cache.borrow_mut().browser_loading_progress_int = progr;

            let b_loading = get_vertical_block_char(p_int as usize, 256);
            let text = format!("<tt>\u{2595}{b_loading}</tt>");

            return (Some(text), None);
        }
        (None, None)
    }
    fn get_label_id(&self) -> u8 {
        LABEL_STATUS_3
    }
}
