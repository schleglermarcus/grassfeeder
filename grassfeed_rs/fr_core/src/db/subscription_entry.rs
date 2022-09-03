use resources::gen_icons;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;

#[allow(dead_code)]
pub const SRC_REPO_ID_MOVING: isize = -3;
#[allow(dead_code)]
pub const SRC_REPO_ID_DELETED: isize = -2;

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubscriptionEntry {
    pub subs_id: isize,
    pub parent_subs_id: isize,
    pub is_folder: bool,
    pub last_selected_msg: isize,
    pub expanded: bool,
    pub deleted: bool,
    pub folder_position: isize,
    pub icon_id: usize,
    ///  timestamp the website compiled the rss file
    pub updated_ext: i64,
    ///  timestamp when we updated this feed from external website
    pub updated_int: i64,
    ///  timestamp when we got the last icon from the website
    pub updated_icon: i64,
    pub display_name: String,
    pub url: String, // xml_url
    pub website_url: String,
    #[serde(skip)]
    pub tree_path: Option<Vec<u16>>,
    #[serde(skip)]
    pub status: usize,
    #[serde(skip)]
    pub num_msg_all_unread: Option<(isize, isize)>,
    #[serde(skip)]
    pub is_dirty: bool,
}

impl SubscriptionEntry {
    pub fn from_new_foldername(display: String, parent_source_repo_id: isize) -> Self {
        SubscriptionEntry {
            subs_id: 0,
            display_name: display,
            is_folder: true,
            url: String::default(),
            icon_id: gen_icons::IDX_08_GNOME_FOLDER_48,
            parent_subs_id: parent_source_repo_id,
            folder_position: 0,
            updated_ext: 0,
            updated_int: 0,
            updated_icon: 0,
            expanded: false,
            website_url: String::default(),
            last_selected_msg: -1,
            num_msg_all_unread: None,
            status: 0,
            tree_path: None,
            is_dirty: true,
            deleted: false,
        }
    }

    pub fn from_new_url(display: String, url_: String) -> Self {
        SubscriptionEntry {
            subs_id: 0,
            display_name: display,
            is_folder: false,
            url: url_,
            icon_id: gen_icons::IDX_05_RSS_FEEDS_GREY_64_D,
            parent_subs_id: 0,
            folder_position: 0,
            updated_ext: 0,
            updated_int: 0, 
            updated_icon: 0,
            expanded: false,
            website_url: String::default(),
            last_selected_msg: -1,
            num_msg_all_unread: None,
            is_dirty: true,
            status: 0,
            tree_path: None,
            deleted: false,
        }
    }

    ///  parent_repo>0
    pub fn isdeleted(&self) -> bool {
        self.parent_subs_id == SRC_REPO_ID_DELETED
            || self.parent_subs_id == SRC_REPO_ID_MOVING
            || self.is_deleted()
    }
}

impl fmt::Debug for SubscriptionEntry {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("SubscriptionEntry")
            .field("id", &self.subs_id)
            .field("PA", &self.parent_subs_id)
            .field("FP", &self.folder_position)
            .field("DEL", &self.deleted)
            .field("FO", &self.is_folder)
            .field("ST", &self.status)
            .field("D", &self.display_name)
            .field("url", &self.url)
            .field("icon", &self.icon_id)
            .field("u_ext", &self.updated_ext)
            .field("u_int", &self.updated_int)
            .field("u_icn", &self.updated_icon)
            .field("XP", &self.expanded)
            .field("web", &self.website_url)
            .finish()
    }
}

impl fmt::Display for SubscriptionEntry {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let title = if self.is_folder { "FO" } else { "SE" };
        let expi: usize = if self.expanded { 1 } else { 0 };
        fmt.debug_struct(title)
            .field("ID", &self.subs_id)
            .field("PA", &self.parent_subs_id)
            .field("FP", &self.folder_position)
            .field("XP", &expi)
            .field("ST", &self.status)
            .finish()
    }
}

pub trait FeedSourceState {
    fn is_expanded(&self) -> bool;
    fn set_expanded(&mut self, new_exp: bool);

    fn is_err_on_fetch(&self) -> bool;

    fn set_err_on_fetch(&mut self, n: bool);
    fn is_fetch_scheduled(&self) -> bool;
    fn set_fetch_scheduled(&mut self, n: bool);

    fn is_fetch_scheduled_jobcreated(&self) -> bool;
    fn set_fetch_scheduled_jobcreated(&mut self, n: bool);

    fn is_fetch_in_progress(&self) -> bool;
    fn set_fetch_in_progress(&mut self, n: bool);

    fn is_deleted(&self) -> bool;
    fn set_deleted(&mut self, n: bool);

    fn check_bitmask(&self, bitmask: usize) -> bool;
    fn change_bitmask(&mut self, bitmask: usize, new_state: bool);
}

#[allow(dead_code)]
impl FeedSourceState for SubscriptionEntry {
    fn is_err_on_fetch(&self) -> bool {
        self.check_bitmask(StatusMask::ErrFetchReq as usize)
    }

    fn set_err_on_fetch(&mut self, n: bool) {
        self.change_bitmask(StatusMask::ErrFetchReq as usize, n)
    }

    fn is_fetch_scheduled(&self) -> bool {
        self.check_bitmask(StatusMask::FetchScheduled as usize)
    }
    fn set_fetch_scheduled(&mut self, n: bool) {
        self.change_bitmask(StatusMask::FetchScheduled as usize, n)
    }

    fn is_fetch_scheduled_jobcreated(&self) -> bool {
        self.check_bitmask(StatusMask::FetchScheduledJobCreated as usize)
    }
    fn set_fetch_scheduled_jobcreated(&mut self, n: bool) {
        self.change_bitmask(StatusMask::FetchScheduledJobCreated as usize, n)
    }

    fn is_fetch_in_progress(&self) -> bool {
        self.check_bitmask(StatusMask::FetchInProgress as usize)
    }

    fn set_fetch_in_progress(&mut self, n: bool) {
        self.change_bitmask(StatusMask::FetchInProgress as usize, n)
    }

    fn is_deleted(&self) -> bool {
        self.deleted
    }
    fn set_deleted(&mut self, n: bool) {
        self.deleted = n
    }

    fn is_expanded(&self) -> bool {
        self.expanded
    }
    fn set_expanded(&mut self, new_exp: bool) {
        self.expanded = new_exp;
    }

    fn check_bitmask(&self, bitmask: usize) -> bool {
        self.status & bitmask == bitmask
    }

    fn change_bitmask(&mut self, bitmask: usize, new_state: bool) {
        let new_st = match new_state {
            true => self.status | bitmask,
            false => self.status & !bitmask,
        };
        if new_st != self.status {
            self.status = new_st;
            self.is_dirty = true;
        }
    }
}

#[allow(dead_code)]
pub enum StatusMask {
    Dirty = 1,
    FetchScheduled = 8,
    FetchScheduledJobCreated = 16,
    FetchInProgress = 32,
    ErrFetchReq = 64,
    ErrIconReq = 128,
    //    FolderExpanded = 64,
    //    Deleted = 128,
}
