use crate::controller::sourcetree::Config;
use crate::db::errors_repo::ErrorRepo;
use crate::db::icon_repo::IconRepo;
use crate::db::subscription_entry::SubscriptionEntry;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_state::FeedSourceState;
use crate::db::subscription_state::ISubscriptionState;
use crate::db::subscription_state::SubsMapEntry;
use crate::db::subscription_state::SubscriptionState;
use gui_layer::abstract_ui::AValue;
use gui_layer::abstract_ui::UIAdapterValueStoreType;
use gui_layer::abstract_ui::UIUpdaterAdapter;
use gui_layer::gui_values::FontAttributes;
use resources::gen_icons;
use resources::id::TREE0_COL_STATUS_EXPANDED;
use std::cell::RefCell;
use std::rc::Rc;

//
pub struct SubscriptionsDisplay {
    pub statemap: Rc<RefCell<SubscriptionState>>,
    pub subscriptionrepo_r: Rc<RefCell<dyn ISubscriptionRepo>>,
    pub gui_val_store: UIAdapterValueStoreType,
    pub iconrepo_r: Rc<RefCell<IconRepo>>,
    pub erro_repo_r: Rc<RefCell<ErrorRepo>>,
    pub config: Rc<RefCell<Config>>,
    pub gui_updater: Rc<RefCell<dyn UIUpdaterAdapter>>,
}

impl SubscriptionsDisplay {
    /// Creates the tree, fills the gui_val_store ,  is recursive.
    pub fn insert_tree_row(&self, localpath: &[u16], parent_subs_id: i32) -> i32 {
        let entries = self
            .subscriptionrepo_r
            .borrow()
            .get_by_parent_repo_id(parent_subs_id as isize);
        entries.iter().enumerate().for_each(|(n, fse)| {
            let mut path: Vec<u16> = Vec::new();
            path.extend_from_slice(localpath);
            path.push(n as u16);
            let subs_map = match self.statemap.borrow().get_state(fse.subs_id) {
                Some(m) => m,
                None => {
                    warn!("no subs_map for id {} {:?}", fse.subs_id, &path);
                    SubsMapEntry::default()
                }
            };
            let treevalues = self.tree_row_to_values(fse, &subs_map);
            (*self.gui_val_store)
                .write()
                .unwrap()
                .insert_tree_item(&path, treevalues.as_slice());
            self.insert_tree_row(&path, fse.subs_id as i32); // recurse
        });
        entries.len() as i32
    }

    /// We overlap the  in-mem Folder-expanded with DB-Folder-Expanded
    pub fn tree_row_to_values(&self, fse: &SubscriptionEntry, su_st: &SubsMapEntry) -> Vec<AValue> {
        let mut tv: Vec<AValue> = Vec::new(); // linked to ObjectTree
        let mut rightcol_text = String::default(); // later:  folder sum stats
        let mut num_msg_unread = 0;
        if let Some((num_all, num_unread)) = su_st.num_msg_all_unread {
            if (*self.config).borrow().display_feedcount_all {
                if num_unread > 0 {
                    rightcol_text = format!("{}/{}", num_unread, num_all);
                } else {
                    rightcol_text = format!("{}", num_all);
                }
            } else {
                rightcol_text = format!("{}", num_unread);
            }
            num_msg_unread = num_unread;
        }
        let mut fs_iconstr: String = String::default();
        if let Some(ie) = self.iconrepo_r.borrow().get_by_index(fse.icon_id as isize) {
            fs_iconstr = ie.icon;
        }
        let mut show_status_icon = false;
        let mut status_icon = gen_icons::ICON_03_ICON_TRANSPARENT_48;

        if su_st.is_fetch_scheduled() || su_st.is_fetch_scheduled_jobcreated() {
            status_icon = gen_icons::ICON_14_ICON_DOWNLOAD_64;
            show_status_icon = true;
        } else if su_st.is_err_on_fetch() {
            status_icon = gen_icons::ICON_32_FLAG_RED_32;
            show_status_icon = true;
        }
        let tp = match &su_st.tree_path {
            Some(tp) => format!("{:?}", &tp),
            None => "".to_string(),
        };
        let mut m_status = su_st.status as u32;
        if fse.expanded {
            m_status |= TREE0_COL_STATUS_EXPANDED;
        }
        let displayname = if fse.display_name.is_empty() {
            String::from("--")
        } else {
            fse.display_name.clone()
        };
        let mut tooltip_a = AValue::None;
        if su_st.is_err_on_fetch() {
            if let Some(last_e) = (*self.erro_repo_r).borrow().get_last_entry(fse.subs_id) {
                // debug!("err-list {}  => {:?}", fse.subs_id, errorlist);
                let mut e_part = last_e.text;
                e_part.truncate(100);
                tooltip_a = AValue::ASTR(e_part);
            }
        }
        if (*self.config).borrow().mode_debug && tooltip_a == AValue::None {
            tooltip_a = AValue::ASTR(format!(
                "{} ST{} X{}  P{:?} I{} L{}",
                fse.subs_id,
                su_st.status,
                match fse.expanded {
                    true => 1,
                    _ => 0,
                },
                tp,
                fse.icon_id,
                fse.last_selected_msg
            ));
        }
        let show_spinner = su_st.is_fetch_in_progress();
        let mut rightcol_visible = !(show_status_icon | show_spinner);
        if !(*self.config).borrow().display_feedcount_all && num_msg_unread == 0 {
            rightcol_visible = false;
        }

        tv.push(AValue::AIMG(fs_iconstr)); // 0
        tv.push(AValue::ASTR(displayname)); // 1:
        tv.push(AValue::ASTR(rightcol_text));
        tv.push(AValue::AIMG(status_icon.to_string()));
        tv.push(AValue::AU32(0)); // 4: is-folder
        tv.push(AValue::AU32(fse.subs_id as u32)); // 5: db-id
        tv.push(AValue::AU32(FontAttributes::to_activation_bits(
            (*self.config).borrow().tree_fontsize as u32,
            num_msg_unread <= 0,
            fse.is_folder,
            false,
        ))); //  6: num_content_unread
        tv.push(AValue::AU32(m_status)); //	7 : status
        tv.push(tooltip_a); //  : 8 tooltip
        tv.push(AValue::ABOOL(show_spinner)); //  : 9	spinner visible
        tv.push(AValue::ABOOL(!show_spinner)); //  : 10	StatusIcon Visible
        tv.push(AValue::ABOOL(rightcol_visible)); //  11: unread-text visible
        tv
    }

    // return: true on success,   false on fail / path check needed
    pub fn tree_update_one(&self, subscr: &SubscriptionEntry, su_st: &SubsMapEntry) -> bool {
        if subscr.isdeleted() {
            warn!("tree_update_one:  is_deleted ! {:?}", subscr);
            return false;
        }
        match &su_st.tree_path {
            Some(t_path) => {
                let treevalues = self.tree_row_to_values(subscr, su_st);
                (*self.gui_val_store)
                    .write()
                    .unwrap()
                    .replace_tree_item(t_path, &treevalues);
                (*self.gui_updater)
                    .borrow()
                    .update_tree_single(0, t_path.as_slice());
                true
            }
            None => {
                warn!(
                    "tree_update_one: no path for id {} <= {:?}",
                    subscr.subs_id, su_st.tree_path
                );
                // self.need_check_fs_paths.replace(true);
                false
            }
        }
    }
    //
}
