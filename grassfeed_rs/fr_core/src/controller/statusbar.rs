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
const ERROR_TEXT_COLOR: &str = "#CC6666";

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
        let mut c = self.cache.borrow_mut();
        c.downloader_kind[threadnr as usize] = kind;
        c.downloader_kind_changed = true;
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
        let old_prog = self.cache.borrow().browser_loading_progress;
        if p != old_prog {
            let mut c = self.cache.borrow_mut();
            c.browser_loading_progress = p;
            c.browser_loading_progress_changed = true;
        }
    }

    pub fn pop_bottom_message(&self) -> Option<String> {
        self.cache.borrow_mut().pop_bottom_message_int()
    }

    pub fn update(&self) {
        let subs_id = self.get_subscription_info();
        self.get_contents_info(subs_id);

        for p in &self.panels {
            let label_id = p.get_label_id();
            let (o_labeltext, o_tooltip) = p.calculate_update(&self);
            let with_tooltip = o_tooltip.is_some();
            let with_label = o_labeltext.is_some();
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

            if with_tooltip {
                (*self.gui_updater).borrow().update_label_markup(label_id);
            } else if with_label {
                (*self.gui_updater).borrow().update_label(label_id);
            }
        }
    }

    // returns subscription_id
    fn get_subscription_info(&self) -> isize {
        let subs_id_old = self.cache.borrow().subscription_id;
        let subscription_id_new: isize;
        let o_subscription = (*self.r_subscriptions_controller)
            .borrow()
            .get_current_selected_subscription();
        let mut subs_is_folder: bool = false;
        let mut subs_updated_int: i64 = 0;
        if let Some((fse, _)) = o_subscription {
            subscription_id_new = fse.subs_id;
            subs_is_folder = subs_is_folder;
            subs_updated_int = fse.updated_int;
        } else {
            subscription_id_new = -1;
        }
        if subscription_id_new != subs_id_old {
            let dl_r_b = self.r_downloader.borrow();
            let mut c = self.cache.borrow_mut();
            c.subscription_id = subscription_id_new;
            c.num_downloader_threads = dl_r_b.get_config().num_downloader_threads;
            c.downloader_stats = dl_r_b.get_statistics();
            c.subscription_is_folder = subs_is_folder;
            c.subscription_last_download_time = subs_updated_int;
            c.subscription_id_changed = true;
        } else {
            let is_changed = self.cache.borrow().subscription_id_changed;
            if is_changed {
                self.cache.borrow_mut().subscription_id_changed = false;
            }
        }
        subscription_id_new
    }

    fn get_contents_info(&self, subscription_id: isize) {
        let content_ids = (*self.r_messages).borrow().get_selected_content_ids();
        let mut new_selected_msg_id = -1;
        if !content_ids.is_empty() {
            new_selected_msg_id = *content_ids.first().unwrap();
        }
        if new_selected_msg_id != self.cache.borrow().selected_msg_id {
            let mut c = self.cache.borrow_mut();
            c.selected_msg_id = new_selected_msg_id;
            c.selected_msg_changed = true;
            c.selected_msg_url = (self.r_browserpane).borrow().get_last_selected_link();
        } else if self.cache.borrow().selected_msg_changed {
            self.cache.borrow_mut().selected_msg_changed = false;
        }
        let subs_state: SubsMapEntry = (*self.r_subscriptions_controller)
            .borrow()
            .get_state(subscription_id)
            .unwrap_or_default();

        if let Some((n_a, n_u)) = subs_state.num_msg_all_unread {
            let old_n_all: isize;
            let old_n_unread: isize;
            let old_is_changed: bool;
            {
                let c = self.cache.borrow();
                old_n_all = c.num_msg_all;
                old_n_unread = c.num_msg_unread;
                old_is_changed = c.num_msg_changed;
            }
            if n_a != old_n_all || n_u != old_n_unread {
                let mut c = self.cache.borrow_mut();
                c.num_msg_all = n_a;
                c.num_msg_unread = n_u;
                c.num_msg_changed = true;
            } else if old_is_changed {
                self.cache.borrow_mut().num_msg_changed = false;
            }
        }
        let new_qsize = (*self.r_downloader).borrow().get_queue_size();
        let new_qsize = new_qsize.0 + new_qsize.1; // queue + threads
        if new_qsize != self.cache.borrow().num_dl_queue_length {
            let mut c = self.cache.borrow_mut();
            c.num_dl_queue_length = new_qsize;
            c.num_dl_queue_length_changed = true;
        } else if self.cache.borrow().num_dl_queue_length_changed {
            self.cache.borrow_mut().num_dl_queue_length_changed = false;
        }
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
            "<span foreground=\"{}\">{}</span>",
            ERROR_TEXT_COLOR,
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
    pub last_fetch_time: i64,
    pub num_downloader_threads: u8,
    /// proc//status/VmRSS  Resident set size, estimation of the current physical memory used by the application
    pub mem_usage_vmrss_bytes: isize,
    pub mode_debug: bool,
    pub bottom_notices: VecDeque<String>,
    //  start-of-display  time,  current message
    pub bottom_notice_current: Option<(i64, String)>,
    pub downloader_stats: [u32; DLKIND_MAX],
    pub db_check_running: bool,
    pub db_check_display_message: String,

    ///  change name
    pub subscription_id: isize,
    pub subscription_id_changed: bool,

    pub subscription_is_folder: bool,
    pub subscription_last_download_time: i64,

    pub num_msg_all: isize,
    pub num_msg_unread: isize,
    pub num_msg_changed: bool,

    //    pub downloader_kind_new: [u8; DOWNLOADER_MAX_NUM_THREADS],
    pub downloader_kind: [u8; DOWNLOADER_MAX_NUM_THREADS],
    pub downloader_kind_changed: bool,

    pub num_dl_queue_length: u16,
    pub num_dl_queue_length_changed: bool,

    pub selected_msg_id: i32,
    pub selected_msg_url: String,
    pub selected_msg_changed: bool,

    pub browser_loading_progress: u8,
    pub browser_loading_progress_changed: bool,
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

    pub fn update_downloader_kind(&mut self, idx: u8, new_dl_kind: u8) {
        if new_dl_kind != self.downloader_kind[idx as usize] {
            self.downloader_kind_changed = true;
        }
        self.downloader_kind[idx as usize] = new_dl_kind;
    }

    pub fn reset_downloader_kind_updated(&mut self) {
        self.downloader_kind_changed = false;
    }
}

struct PanelLeft {}

impl OnePanel for PanelLeft {
    fn calculate_update(&self, statusbar: &StatusBar) -> (Option<String>, Option<String>) {
        // label-1
        let timestamp_now: i64 = timestamp_now();
        let mut need_update_1: bool = false;
        let num_msg_all: isize;
        let num_msg_unread: isize;
        let num_msg_changed: bool;
        let is_folder: bool;
        let subs_id_changed: bool;
        let downloader_kind: [u8; DOWNLOADER_MAX_NUM_THREADS];
        let n_threads: usize;
        let downloader_kind_changed: bool;
        let downloader_queue_length: u16;
        let downloader_queue_changed: bool;
        {
            let c = statusbar.cache.borrow();
            is_folder = c.subscription_is_folder;
            subs_id_changed = c.subscription_id_changed;
            num_msg_all = c.num_msg_all;
            num_msg_unread = c.num_msg_unread;
            num_msg_changed = c.num_msg_changed;
            downloader_kind = c.downloader_kind;
            n_threads = c.num_downloader_threads as usize;
            downloader_kind_changed = c.downloader_kind_changed;
            downloader_queue_length = c.num_dl_queue_length;
            downloader_queue_changed = c.num_dl_queue_length_changed;
        }
        if num_msg_changed || downloader_kind_changed || downloader_queue_changed {
            need_update_1 = true;
        }
        //  debug!("PanelLeft:1   {num_msg_changed}  {downloader_kind_changed}     {downloader_queue_changed}   vert {} " ,  !is_folder && subs_id_changed );
        let mut block_vertical: char = ' ';
        if !is_folder && subs_id_changed {
            let fs_conf = statusbar.r_subscriptions_controller.borrow().get_config();
            let last_fetch_time = statusbar.cache.borrow().subscription_last_download_time;
            let interval_s = (*fs_conf).borrow().get_interval_seconds();
            let elapsed: i64 = std::cmp::min(timestamp_now - (last_fetch_time), interval_s);
            block_vertical = get_vertical_block_char(elapsed as usize, interval_s as usize);
            need_update_1 = true;
        }

        if !need_update_1 {
            return (None, None);
        }
        let mut downloader_display: String = String::default();
        for a in 0..n_threads {
            let nc = dl_char_for_kind(downloader_kind[a]);
            downloader_display.push(nc);
        }
        let unread_all = format!("{:5} / {:5}", num_msg_unread, num_msg_all);
        statusbar.cache.borrow_mut().reset_downloader_kind_updated();
        let t_popup = format!(
            "q{}  {}MB",
            downloader_queue_length,
            statusbar.cache.borrow().mem_usage_vmrss_bytes / 1048576
        );

        let queue_display_max = DOWNLOADER_MAX_NUM_THREADS << 1; // double amount of threads shall display full char
        let dl_queue_char = get_vertical_block_char(
            usize::min(downloader_queue_length as usize, queue_display_max),
            queue_display_max,
        );
        let msg1 = format!(   "<tt>{dl_queue_char} {downloader_display}  {unread_all}    \u{2595}{block_vertical}\u{258F}</tt>"   );
        // debug!("PanelLeft:3    {msg1}  {t_popup}  ");
        (Some(msg1), Some(t_popup))
    }
    fn get_label_id(&self) -> u8 {
        LABEL_STATUS_1
    }
}

struct PanelMiddle {}
impl OnePanel for PanelMiddle {
    fn calculate_update(&self, statusbar: &StatusBar) -> (Option<String>, Option<String>) {
        let selected_msg_url: String;
        let selected_msg_changed: bool;
        let bottom_notice_current: Option<(i64, String)>;
        {
            let c = statusbar.cache.borrow();
            selected_msg_url = c.selected_msg_url.clone();
            selected_msg_changed = c.selected_msg_changed;
            bottom_notice_current = c.bottom_notice_current.clone();
        }
        let mut feed_src_link = String::default();
        let o_subscription = (*statusbar.r_subscriptions_controller)
            .borrow()
            .get_current_selected_subscription();
        if let Some((fse, _)) = o_subscription {
            feed_src_link.clone_from(&fse.url);
        }
        let mut need_update_2: bool = false;

        if selected_msg_changed {
            need_update_2 = true;
        }
        let mut longtext = if selected_msg_url.is_empty() {
            string_escape_url(feed_src_link)
        } else {
            string_escape_url(selected_msg_url.clone())
        };

        if let Some((ts, msg)) = bottom_notice_current {
            let timestamp_now: i64 = timestamp_now();
            if timestamp_now > ts + BOTTOM_MSG_SHOW_TIME_S as i64 {
                statusbar.cache.borrow_mut().bottom_notice_current = None;
            } else {
                longtext = StatusBar::error_formatter(msg.to_string());
            }
            need_update_2 = true;
        } else {
            if let Some(_n_msg) = statusbar.pop_bottom_message() {
                need_update_2 = true;
            }
        }
        if !need_update_2 {
            return (None, None);
        }
        return (Some(longtext), Some(String::from("Toooltip !!")));
    }

    fn get_label_id(&self) -> u8 {
        LABEL_STATUS_2
    }
}

struct PanelRight {}
impl OnePanel for PanelRight {
    fn calculate_update(&self, statusbar: &StatusBar) -> (Option<String>, Option<String>) {
        let progr: u8;
        let progr_changed: bool;
        {
            let c = statusbar.cache.borrow();
            progr = c.browser_loading_progress;
            progr_changed = c.browser_loading_progress_changed;
        }
        if progr_changed {
            statusbar
                .cache
                .borrow_mut()
                .browser_loading_progress_changed = false;
            let b_loading = get_vertical_block_char(progr as usize, 256);
            let text = format!("<tt>\u{2595}{b_loading}</tt>");
            return (Some(text), None);
        }
        (None, None)
    }
    fn get_label_id(&self) -> u8 {
        LABEL_STATUS_3
    }
}
