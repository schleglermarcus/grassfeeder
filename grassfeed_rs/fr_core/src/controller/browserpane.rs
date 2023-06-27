use rust_i18n::t;

use crate::config::configmanager::ConfigManager;
use crate::controller::contentlist::FeedContents;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::ui_select::gui_context::GuiContext;
use crate::util;
use crate::util::string_escape_url;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use gui_layer::abstract_ui::UIAdapterValueStoreType;
use gui_layer::abstract_ui::UIUpdaterAdapter;
use gui_layer::gui_values::PropDef;
use resources::id::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::rc::Weak;

const BROWSER_ZOOM_LIMIT_UPPER: u32 = 300;
const BROWSER_ZOOM_LIMIT_LOWER: u32 = 20;

const WEBVIEW_REGULAR: u8 = 0;
const WEBVIEW_PRELOAD: u8 = 1;

pub trait IBrowserPane {
    fn switch_browsertab_content(
        &self,
        repo_id: i32,
        title: String,
        co_au_ca: Option<(String, String, String)>,
    );
    fn browser_pre_load(&self, msg_id: i32, co_au_ca_su: Option<(String, String, String)>);

    fn get_config(&self) -> Config;
    fn set_conf_browser_bg(&mut self, c: u32);
    fn get_last_selected_link(&self) -> String;
    fn display_short_help(&self);
    fn set_browser_zoom(&self, cmd: BrowserZoomCommand);
}

#[derive(Debug)]
pub enum BrowserZoomCommand {
    None,
    ZoomIn,
    ZoomOut,
    ZoomDefault,
}

pub struct BrowserPane {
    configmanager_r: Rc<RefCell<ConfigManager>>,
    feedcontents_w: Weak<RefCell<FeedContents>>, // YY
    messagesrepo_r: Rc<RefCell<dyn IMessagesRepo>>,
    subscriptionrepo_r: Rc<RefCell<dyn ISubscriptionRepo>>,
    gui_updater: Rc<RefCell<dyn UIUpdaterAdapter>>,
    gui_val_store: UIAdapterValueStoreType,
    config: Config,
    last_selected_link_text: RefCell<String>,
    last_zoom_level_percent: RefCell<u32>,
}

impl BrowserPane {
    pub fn new(ac: &AppContext) -> Self {
        let gc_r = (*ac).get_rc::<GuiContext>().unwrap();
        let u_a = (*gc_r).borrow().get_updater_adapter();
        let v_s_a = (*gc_r).borrow().get_values_adapter();
        let cm_r = (*ac).get_rc::<ConfigManager>().unwrap();
        BrowserPane {
            gui_updater: u_a,
            gui_val_store: v_s_a,
            configmanager_r: cm_r,
            config: Config::default(),
            last_selected_link_text: RefCell::new(String::default()),
            messagesrepo_r: (*ac).get_rc::<MessagesRepo>().unwrap(),
            feedcontents_w: Weak::new(),
            subscriptionrepo_r: (*ac).get_rc::<SubscriptionRepo>().unwrap(),
            last_zoom_level_percent: RefCell::new(100),
        }
    }

    fn create_browser_dir(&mut self) {
        if let Some(browserdir) = (*self.configmanager_r)
            .borrow()
            .get_sys_val(&PropDef::BrowserDir.to_string())
        {
            let existing = std::path::Path::new(&browserdir).is_dir();
            if !existing {
                match std::fs::create_dir(browserdir.clone()) {
                    Ok(_) => (),
                    Err(e) => {
                        error!("could not create browser dir {} {:?}", &browserdir, e);
                    }
                }
            }
        } else {
            error!("config is missing {}", PropDef::BrowserDir.to_string());
        }
    }

    fn set_browser_contents_html(&self, msg: String, webview_idx: u8) {
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_web_view_text(webview_idx, msg);
        (*self.gui_updater).borrow().update_web_view(webview_idx);
    }

    /// load plain text into the browser display
    fn set_browser_contents_plain(&self, msg: String) {
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_web_view_text(0, msg);
        (*self.gui_updater).borrow().update_web_view_plain(0);
    }

    fn set_browser_info_area(
        &self,
        link_title: String,
        link_url: String,
        msg_date: String,
        msg_author: String,
        msg_categories: String,
        subscr_title: String,
    ) {
        if false {
            (*self.gui_val_store).write().unwrap().set_linkbutton_text(
                LINKBUTTON_BROWSER_TITLE,
                (link_title.clone(), link_url.clone()),
            );
            (*self.gui_updater)
                .borrow()
                .update_linkbutton(LINKBUTTON_BROWSER_TITLE);
        }
        let linktext = format!(
            "<a href=\"{}\">{}</a>",
            string_escape_url(link_url),
            string_escape_url(link_title)
        );
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_label_text(LABEL_BROWSER_ENTRY_LINK, linktext);
        (*self.gui_updater)
            .borrow()
            .update_label_markup(LABEL_BROWSER_ENTRY_LINK);
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_label_text(LABEL_BROWSER_SUBSCRIPTION, subscr_title);
        (*self.gui_updater)
            .borrow()
            .update_label(LABEL_BROWSER_SUBSCRIPTION);
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_label_text(LABEL_BROWSER_MSG_DATE, msg_date);
        (*self.gui_updater)
            .borrow()
            .update_label(LABEL_BROWSER_MSG_DATE);
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_label_text(LABEL_BROWSER_MSG_AUTHOR, msg_author);
        (*self.gui_updater)
            .borrow()
            .update_label(LABEL_BROWSER_MSG_AUTHOR);
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_label_text(LABEL_BROWSER_MSG_CATEGORIES, msg_categories);
        (*self.gui_updater)
            .borrow()
            .update_label(LABEL_BROWSER_MSG_CATEGORIES);
    }
}

impl IBrowserPane for BrowserPane {
    fn switch_browsertab_content(
        &self,
        msg_id: i32,
        title: String,
        co_au_ca_su: Option<(String, String, String)>,
    ) {
        if msg_id < 0 {
            error!("switch_browsertab_content_r  repo_id<0");
            return;
        }
        let o_msg = (*self.messagesrepo_r)
            .borrow()
            .get_by_index(msg_id as isize);
        if o_msg.is_none() {
            return;
        }
        let message = o_msg.unwrap();
        let mut content = String::default();
        let mut author = String::default();
        let mut categories = String::default();
        let mut su_title = String::default();
        if let Some(triplet) = co_au_ca_su {
            (content, author, categories) = triplet;
        }
        if let Some(sub_e) = (self.subscriptionrepo_r)
            .borrow()
            .get_by_index(message.subscription_id)
        {
            su_title = sub_e.display_name;
        }
        let mut display = title;
        if let Some(_pos) = display.find("http") {
            display = display.split("http").next().unwrap().to_string();
            display = display.trim().to_string();
        }
        self.last_selected_link_text.replace(message.link.clone()); //;
        let srcdate = util::db_time_to_display(message.entry_src_date);
        self.set_browser_contents_html(content, WEBVIEW_REGULAR);
        self.set_browser_info_area(display, message.link, srcdate, author, categories, su_title)
    }

    fn browser_pre_load(&self, msg_id: i32, co_au_ca_su: Option<(String, String, String)>) {
        if msg_id < 0 {
            error!("browser_pre_load  repo_id<0");
            return;
        }
        let o_msg = (*self.messagesrepo_r)
            .borrow()
            .get_by_index(msg_id as isize);
        if o_msg.is_none() {
            return;
        }
        let mut content = String::default();
        if let Some(triplet) = co_au_ca_su {
            (content, _, _) = triplet;
        }
        // trace!(            "browser_pre_load : {msg_id}  length_of_content:{} ",            content.len()        );
        self.set_browser_contents_html(content, WEBVIEW_PRELOAD);
    }

    fn get_config(&self) -> Config {
        self.config.clone()
    }

    fn set_conf_browser_bg(&mut self, c: u32) {
        self.config.browser_bg = c as u8;
        (*self.configmanager_r)
            .borrow()
            .set_val(&PropDef::BrowserBackgroundLevel.to_string(), c.to_string());
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_gui_property(PropDef::BrowserBackgroundLevel, c.to_string());

        (*self.gui_updater)
            .borrow()
            .update_web_view(WEBVIEW_REGULAR);
    }

    fn get_last_selected_link(&self) -> String {
        self.last_selected_link_text.borrow().clone()
    }

    fn display_short_help(&self) {
        self.set_browser_info_area(
            String::default(),
            String::default(),
            String::default(),
            String::default(),
            String::default(),
            String::default(),
        );
        self.set_browser_contents_plain(t!("M_SHORTHELP_TEXT"));
    }

    fn set_browser_zoom(&self, cmd: BrowserZoomCommand) {
        let cur_zoom: u32 = *self.last_zoom_level_percent.borrow();
        let mut new_zoom: u32 = 100;
        match cmd {
            BrowserZoomCommand::ZoomIn => new_zoom = cur_zoom * 110 / 100,
            BrowserZoomCommand::ZoomOut => new_zoom = cur_zoom * 90 / 100,
            _ => (),
        }
        new_zoom = std::cmp::min(new_zoom, BROWSER_ZOOM_LIMIT_UPPER);
        new_zoom = std::cmp::max(new_zoom, BROWSER_ZOOM_LIMIT_LOWER);
        self.last_zoom_level_percent.replace(new_zoom);
        if new_zoom != cur_zoom {
            (*self.gui_val_store)
                .write()
                .unwrap()
                .set_gui_property(PropDef::BrowserZoomPercent, new_zoom.to_string());
            (*self.gui_updater).borrow().update_web_view(0);
        }
    }
}

impl Buildable for BrowserPane {
    type Output = BrowserPane;
    fn build(conf: Box<dyn BuildConfig>, appcontext: &AppContext) -> Self::Output {
        let mut bp = BrowserPane::new(appcontext);
        if let Some(i) = conf.get_int(&PropDef::BrowserBackgroundLevel.to_string()) {
            bp.config.browser_bg = i as u8;
        } else {
            bp.config.browser_bg = 64;
        }
        bp
    }
}

impl StartupWithAppContext for BrowserPane {
    fn startup(&mut self, ac: &AppContext) {
        self.feedcontents_w = Rc::downgrade(&(*ac).get_rc::<FeedContents>().unwrap());
        self.create_browser_dir();
    }
}

#[derive(Default, Clone, Debug)]
pub struct Config {
    pub browser_bg: u8,
}

//------------------------------------------------------
