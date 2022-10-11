use crate::config::configmanager::ConfigManager;
use crate::controller::contentlist::FeedContents;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::timer::Timer;
use crate::ui_select::gui_context::GuiContext;
use crate::util;
use crate::util::string_escape_url;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use context::TimerEvent;
use context::TimerReceiver;
use gui_layer::abstract_ui::UIAdapterValueStoreType;
use gui_layer::abstract_ui::UIUpdaterAdapter;
use gui_layer::gui_values::PropDef;
use resources::id::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::rc::Weak;

pub trait IBrowserPane {
    fn switch_browsertab_content(
        &self,
        repo_id: i32,
        title: String,
        co_au_ca: Option<(String, String, String)>,
    );

    fn get_config(&self) -> Config;
    fn set_conf_browser_bg(&mut self, c: u32);
    fn get_last_selected_link(&self) -> String;
    fn display_short_help(&self);
}

pub struct BrowserPane {
    timer_r: Rc<RefCell<Timer>>,
    configmanager_r: Rc<RefCell<ConfigManager>>,
    gui_updater: Rc<RefCell<dyn UIUpdaterAdapter>>,
    gui_val_store: UIAdapterValueStoreType,
    config: Config,
    last_selected_link_text: RefCell<String>,
    messagesrepo_r: Rc<RefCell<dyn IMessagesRepo>>,
    feedcontents_w: Weak<RefCell<FeedContents>>, // YY
}

impl BrowserPane {
    pub fn new(ac: &AppContext) -> Self {
        let gc_r = (*ac).get_rc::<GuiContext>().unwrap();
        let u_a = (*gc_r).borrow().get_updater_adapter();
        let v_s_a = (*gc_r).borrow().get_values_adapter();
        let cm_r = (*ac).get_rc::<ConfigManager>().unwrap();
        BrowserPane {
            timer_r: (*ac).get_rc::<Timer>().unwrap(),
            gui_updater: u_a,
            gui_val_store: v_s_a,
            configmanager_r: cm_r,
            config: Config::default(),
            last_selected_link_text: RefCell::new(String::default()),
            messagesrepo_r: (*ac).get_rc::<MessagesRepo>().unwrap(),
            feedcontents_w: Weak::new(),
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
            (*self.configmanager_r)
                .borrow()
                .debug_dump("create_browser_dir");
        }
    }

    fn set_browser_contents_html(&self, msg: String) {
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_web_view_text(0, msg);
        (*self.gui_updater).borrow().update_web_view(0);
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
        repo_id: i32,
        title: String,
        co_au_ca: Option<(String, String, String)>,
    ) {
        if repo_id < 0 {
            error!("switch_browsertab_content_r  repo_id<0");
            return;
        }
        let o_msg = (*self.messagesrepo_r)
            .borrow()
            .get_by_index(repo_id as isize);

        if o_msg.is_none() {
            return;
        }
        let fce = o_msg.unwrap();
        let mut content = String::default();
        let mut author = String::default();
        let mut categories = String::default();
        if let Some(triplet) = co_au_ca {
            (content, author, categories) = triplet;
        }
        let mut display = title;
        if let Some(_pos) = display.find("http") {
            display = display.split("http").next().unwrap().to_string();
            display = display.trim().to_string();
        }
        self.last_selected_link_text.replace(fce.link.clone()); //;
        let srcdate = util::db_time_to_display(fce.entry_src_date);
        /*
                (*self.gui_val_store)
                    .write()
                    .unwrap()
                    .set_web_view_text(0, content);
                (*self.gui_updater).borrow().update_web_view(0);
        */
        self.set_browser_contents_html(content);
        /*
                (*self.gui_val_store)
                    .write()
                    .unwrap()
                    .set_linkbutton_text(LINKBUTTON_BROWSER_TITLE, (display, fce.link));
                (*self.gui_updater)
                    .borrow()
                    .update_linkbutton(LINKBUTTON_BROWSER_TITLE);
                (*self.gui_val_store)
                    .write()
                    .unwrap()
                    .set_label_text(LABEL_BROWSER_MSG_DATE, srcdate);
                (*self.gui_updater)
                    .borrow()
                    .update_label(LABEL_BROWSER_MSG_DATE);
                (*self.gui_val_store)
                    .write()
                    .unwrap()
                    .set_label_text(LABEL_BROWSER_MSG_AUTHOR, author);
                (*self.gui_updater)
                    .borrow()
                    .update_label(LABEL_BROWSER_MSG_AUTHOR);
                (*self.gui_val_store)
                    .write()
                    .unwrap()
                    .set_label_text(LABEL_BROWSER_MSG_CATEGORIES, categories);
                (*self.gui_updater)
                    .borrow()
                    .update_label(LABEL_BROWSER_MSG_CATEGORIES);
        */
        self.set_browser_info_area(display, fce.link, srcdate, author, categories)
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

        (*self.gui_updater).borrow().update_web_view(0);
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
        );
        self.set_browser_contents_plain(t!("M_SHORTHELP_TEXT"));
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
        let _browserpane_r = ac.get_rc::<BrowserPane>().unwrap();
        if false {
            let mut _t = (*self.timer_r).borrow_mut();
            // t.register(&TimerEvent::Timer1s, fc_r.clone());
        }
    }
}

impl TimerReceiver for BrowserPane {
    fn trigger(&mut self, _event: &TimerEvent) {}
}

#[derive(Default, Clone, Debug)]
pub struct Config {
    pub browser_bg: u8,
    // pub cache_cleanup: bool,
}

//------------------------------------------------------
