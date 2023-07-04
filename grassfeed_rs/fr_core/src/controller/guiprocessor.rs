use crate::config::configmanager::ConfigManager;
use crate::controller::browserpane;
use crate::controller::browserpane::BrowserZoomCommand;
use crate::controller::browserpane::IBrowserPane;
use crate::controller::contentdownloader;
use crate::controller::contentdownloader::IDownloader;
use crate::controller::contentlist::FeedContents;
use crate::controller::contentlist::IContentList;
use crate::controller::contentlist::ListMoveCommand;
use crate::controller::isourcetree::ISourceTreeController;
use crate::controller::sourcetree::SJob;
use crate::controller::sourcetree::SourceTreeController;
use crate::controller::statusbar::StatusBar;
use crate::controller::subscriptionmove::ISubscriptionMove;
use crate::controller::subscriptionmove::SubscriptionMove;
use crate::controller::timer::ITimer;
use crate::controller::timer::Timer;
use crate::controller::timer::TimerJob;
use crate::db::errors_repo::ErrorRepo;
use crate::db::icon_repo::IconEntry;
use crate::db::icon_repo::IconRepo;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::db::subscription_state::ISubscriptionState;
use crate::db::subscription_state::StatusMask;
use crate::opml::opmlreader::OpmlReader;
use crate::ui_select::gui_context::GuiContext;
use crate::ui_select::select::ui_select;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use context::TimerEvent;
use context::TimerReceiver;
use flume::Receiver;
use flume::Sender;
use gui_layer::abstract_ui::AValue;
use gui_layer::abstract_ui::BrowserEventType;
use gui_layer::abstract_ui::GuiEvents;
use gui_layer::abstract_ui::GuiRunner;
use gui_layer::abstract_ui::UIAdapterValueStoreType;
use gui_layer::abstract_ui::UIUpdaterAdapter;
use gui_layer::abstract_ui::UIUpdaterMarkWidgetType;
use gui_layer::gui_values::KeyCodes;
use gui_layer::gui_values::PropDef;
use resources::gen_icons;
use resources::id::DIALOG_ABOUT;
use resources::id::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::mem::Discriminant;
use std::rc::Rc;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Job {
    StartApplication,
    StopApplication,
    ReadOpmlFile(String),
    /// text_view-id
    UpdateTextView(u8),
    UpdateWebView(u8),
    UpdateTextEntry(u8),
    /// source_repo_id
    SwitchContentList(isize),
    UpdateLabel(u8),
    NotifyConfigChanged,
    /// thread-nr,  job-kind
    DownloaderJobStarted(u8, u8),
    /// thread-nr, job-kind, elapsed_ms , job-description
    DownloaderJobFinished(isize, u8, u8, u32, String),
    CheckFocusMarker(u8),
    AddBottomDisplayErrorMessage(String),
}

const JOBQUEUE_SIZE: usize = 100;
const TREE_PANE1_MIN_WIDTH: i32 = 100;

#[allow(dead_code)]
pub struct GuiProcessor {
    job_queue_receiver: Receiver<Job>,
    job_queue_sender: Sender<Job>,
    gui_val_store: UIAdapterValueStoreType,
    configmanager_r: Rc<RefCell<ConfigManager>>,
    timer_r: Rc<RefCell<dyn ITimer>>,
    timer_sender: Option<Sender<TimerJob>>,
    gui_updater: Rc<RefCell<dyn UIUpdaterAdapter>>,
    gui_runner: Rc<RefCell<dyn GuiRunner>>,
    gui_context_r: Rc<RefCell<GuiContext>>,
    feedsources_r: Rc<RefCell<dyn ISourceTreeController>>,
    contentlist_r: Rc<RefCell<dyn IContentList>>,
    downloader_r: Rc<RefCell<dyn IDownloader>>,
    browserpane_r: Rc<RefCell<dyn IBrowserPane>>,
    erro_repo_r: Rc<RefCell<ErrorRepo>>,
    subscriptionmove_r: Rc<RefCell<dyn ISubscriptionMove>>,
    subscriptionrepo_r: Rc<RefCell<dyn ISubscriptionRepo>>,
    iconrepo_r: Rc<RefCell<IconRepo>>,
    statusbar: RefCell<StatusBar>,
    focus_by_tab: RefCell<FocusByTab>,
    currently_minimized: RefCell<bool>,
    event_handler_map: HashMap<Discriminant<GuiEvents>, Box<dyn HandleSingleEvent>>,
}

pub trait HandleSingleEvent {
    fn handle(&self, _ev: GuiEvents, _gp: &GuiProcessor) {}
}

impl GuiProcessor {
    pub fn new(ac: &AppContext) -> Self {
        let (q_s, q_r) = flume::bounded::<Job>(JOBQUEUE_SIZE);
        let guicontex_r = (*ac).get_rc::<GuiContext>().unwrap();
        let gui_u_a = (*guicontex_r).borrow().get_updater_adapter();
        let gui_v_a = (*guicontex_r).borrow().get_values_adapter();
        let guirunner = (*guicontex_r).borrow().get_gui_runner();
        let dl_r = (*ac).get_rc::<contentdownloader::Downloader>().unwrap();
        let err_rep = (*ac).get_rc::<ErrorRepo>().unwrap();
        let status_bar = RefCell::new(StatusBar::new(
            (*ac).get_rc::<SourceTreeController>().unwrap(),
            dl_r.clone(),
            gui_u_a.clone(),
            (*ac).get_rc::<FeedContents>().unwrap(),
            (*ac).get_rc::<browserpane::BrowserPane>().unwrap(),
            gui_v_a.clone(),
        ));

        GuiProcessor {
            subscriptionrepo_r: (*ac).get_rc::<SubscriptionRepo>().unwrap(),
            configmanager_r: (*ac).get_rc::<ConfigManager>().unwrap(),
            feedsources_r: (*ac).get_rc::<SourceTreeController>().unwrap(),
            contentlist_r: (*ac).get_rc::<FeedContents>().unwrap(),
            iconrepo_r: (*ac).get_rc::<IconRepo>().unwrap(),
            timer_r: (*ac).get_rc::<Timer>().unwrap(),
            browserpane_r: (*ac).get_rc::<browserpane::BrowserPane>().unwrap(),
            job_queue_sender: q_s,
            job_queue_receiver: q_r,
            timer_sender: None,
            gui_updater: gui_u_a,
            gui_val_store: gui_v_a,
            gui_runner: guirunner,
            downloader_r: dl_r,
            gui_context_r: guicontex_r,
            erro_repo_r: err_rep,
            subscriptionmove_r: (*ac).get_rc::<SubscriptionMove>().unwrap(),
            focus_by_tab: RefCell::new(FocusByTab::None),
            currently_minimized: RefCell::new(false),
            statusbar: status_bar,
            event_handler_map: Default::default(),
        }
    }

    pub fn process_event(&self) {
        let mut ev_set: HashSet<GuiEvents> = HashSet::new();
        let receiver = (*self.gui_runner).borrow().get_event_receiver();
        loop {
            let ev = receiver.get_event_try();
            match ev {
                GuiEvents::None => break,
                _ => {
                    ev_set.insert(ev);
                }
            }
        }
        let mut list_row_activated_map: HashMap<i32, i32> = HashMap::default();
        for ev in ev_set {
            match ev {
                GuiEvents::None => {}
                GuiEvents::ListRowActivated(_list_idx, list_position, msg_id) => {
                    list_row_activated_map.insert(msg_id, list_position);
                }
                _ => {
                    if let Some(handler_b) =
                        self.event_handler_map.get(&std::mem::discriminant(&ev))
                    {
                        let ev_ident = (Instant::now(), format!("{:?}", &ev));
                        handler_b.handle(ev, self);
                        let elapsed_m = ev_ident.0.elapsed().as_millis();
                        if elapsed_m > 160 {
                            debug!("EV  {}   took {:?}", ev_ident.1, elapsed_m);
                        }
                    } else {
                        warn!("EV not found: {:?}", &ev);
                    }
                }
            }
        }
        if !list_row_activated_map.is_empty() {
            self.focus_by_tab.replace(FocusByTab::FocusMessages);
            (*self.contentlist_r)
                .borrow()
                .process_list_row_activated(&list_row_activated_map);
        }
    }

    /// is run by  the timer
    pub fn process_jobs(&self) {
        let mut job_list: Vec<Job> = Vec::new();
        while let Ok(job) = self.job_queue_receiver.try_recv() {
            if !job_list.contains(&job) {
                job_list.push(job);
            }
        }
        for job in job_list {
            let now = Instant::now();
            let job2 = job.clone();
            match job {
                Job::StartApplication => {
                    (*self.gui_runner).borrow().start();
                    self.store_default_icons();
                    (*self.gui_context_r)
                        .borrow_mut()
                        .set_window_title(String::default());
                    (*self.gui_val_store)
                        .write()
                        .unwrap()
                        .set_window_icon(gen_icons::ICON_04_GRASS_CUT_2.to_string());
                    (*self.gui_updater).borrow().update_window_icon();
                }
                Job::StopApplication => {
                    match self
                        .timer_sender
                        .as_ref()
                        .unwrap()
                        .try_send(TimerJob::Shutdown)
                    {
                        Ok(_) => (),
                        Err(e) => error!("GP: StopApplication, cannot send to Timer {:?}", e),
                    }
                    (*self.gui_runner).borrow_mut().stop();
                }
                Job::SwitchContentList(feed_source_id) => {
                    (*self.contentlist_r)
                        .borrow()
                        .update_message_list_(feed_source_id);
                    (*self.gui_updater).borrow().update_list(0);
                }
                Job::UpdateTextView(t_v_id) => {
                    (*self.gui_updater).borrow().update_text_view(t_v_id);
                }
                Job::UpdateWebView(t_v_id) => {
                    (*self.gui_updater).borrow().update_web_view(t_v_id);
                }
                Job::UpdateTextEntry(nr) => {
                    (*self.gui_updater).borrow().update_text_entry(nr);
                }
                Job::UpdateLabel(nr) => {
                    (*self.gui_updater).borrow().update_label(nr);
                }
                Job::NotifyConfigChanged => {
                    (*self.contentlist_r).borrow_mut().notify_config_update();
                    (*self.feedsources_r).borrow_mut().notify_config_update();
                }
                Job::DownloaderJobStarted(threadnr, kind) => {
                    self.statusbar.borrow_mut().downloader_kind_new[threadnr as usize] = kind;
                }
                Job::DownloaderJobFinished(subs_id, threadnr, _kind, elapsed_ms, description) => {
                    // if kind == 6 {                        trace!("browser_launch:{}ms {}", elapsed_ms, &description);                    }
                    if elapsed_ms > 1000 && subs_id > 0 {
                        (*self.erro_repo_r).borrow().add_error(
                            subs_id,
                            elapsed_ms as isize,
                            String::default(),
                            description,
                        );
                    }
                    self.statusbar.borrow_mut().downloader_kind_new[threadnr as usize] = 0;
                }
                Job::CheckFocusMarker(num) => {
                    if num > 0 {
                        self.addjob(Job::CheckFocusMarker(num - 1))
                    } else {
                        self.switch_focus_marker(false);
                    }
                }
                Job::AddBottomDisplayErrorMessage(msg) => {
                    self.statusbar.borrow_mut().bottom_notices.push_back(msg);
                }
                _ => {
                    warn!("other job! {:?}", &job);
                }
            }
            let elapsedms = now.elapsed().as_millis();
            if elapsedms > 100 {
                warn!("JOB {:?} took {:?}ms", &job2, elapsedms);
            }
        }
    }

    pub fn addjob(&self, nj: Job) {
        if self.job_queue_sender.is_full() {
            warn!(
                "GP job queue full, size {}.  Skipping  {:?}",
                JOBQUEUE_SIZE, nj
            );
        } else {
            self.job_queue_sender.send(nj).unwrap();
        }
    }

    // single entries:  53ms		combined:  39ms
    pub fn store_default_icons(&self) {
        gen_icons::ICON_LIST
            .iter()
            .enumerate()
            .map(|(num, ico)| IconEntry {
                icon_id: num as isize,
                icon: ico.to_string(),
            })
            .for_each(|e| {
                let _r = (*self.iconrepo_r.borrow()).store_entry(&e);
            });
    }

    fn start_settings_dialog(&self) {
        let sources_conf = (*self.feedsources_r).borrow().get_config();
        if (sources_conf).borrow().feeds_fetch_interval_unit == 0 {
            (sources_conf).borrow_mut().feeds_fetch_interval_unit = 3; // set to days if it was not set before
        }
        let downloader_conf = (*self.downloader_r).borrow().get_config();
        let contentlist_conf = (*self.contentlist_r).borrow().get_config();
        let browser_conf = (*self.browserpane_r).borrow().get_config();
        let fontsize_manual_enable = (*self.gui_val_store)
            .read()
            .unwrap()
            .get_gui_property_or(PropDef::GuiFontSizeManualEnable, "false".to_string())
            .parse::<bool>()
            .unwrap();
        let fontsize_manual = (*self.gui_val_store)
            .read()
            .unwrap()
            .get_gui_int_or(PropDef::GuiFontSizeManual, 10) as u32;
        let browser_cache_clear = (self.configmanager_r)
            .borrow()
            .get_val_bool(&PropDef::BrowserClearCache.to_string());
        let databases_cleanup = (self.configmanager_r)
            .borrow()
            .get_val_bool(contentdownloader::CONF_DATABASES_CLEANUP);
        let systray_enable = self.is_systray_enabled();
        let dd: Vec<AValue> = vec![
            AValue::ABOOL((sources_conf).borrow().feeds_fetch_at_start), // 0 : FetchFeedsOnStart
            AValue::AU32((sources_conf).borrow().feeds_fetch_interval),  // 1 UpdateFeeds Cardinal
            AValue::AU32((sources_conf).borrow().feeds_fetch_interval_unit), // 2 UpdateFeeds Unit:  1:minutes  2:hours  3:days
            AValue::AU32(downloader_conf.num_downloader_threads as u32), // 3 Web Fetcher Threads
            AValue::AI32(contentlist_conf.focus_policy as i32),          // 4 Message Focus Policy
            AValue::ABOOL((sources_conf).borrow().display_feedcount_all), // 5 : DisplayCountOfAllFeeds
            AValue::AU32(contentlist_conf.message_keep_count as u32),     // 6 Messages Keep Count
            AValue::ABOOL(fontsize_manual_enable), // 7 : FontSizeManualEnable
            AValue::AU32(fontsize_manual),         // 8 : Font size Manual
            AValue::AU32(browser_conf.browser_bg as u32), // 9 : Browser_BG
            AValue::ABOOL(browser_cache_clear),    // 10 : Browser Cache Cleanup
            AValue::ABOOL(databases_cleanup),      // 11 : Cleanup-on-start
            AValue::ABOOL(systray_enable),         // 12 : Systray enable
        ];
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_dialog_data(DIALOG_SETTINGS, &dd);
        (*self.gui_updater).borrow().update_dialog(DIALOG_SETTINGS);
        (*self.gui_updater).borrow().show_dialog(DIALOG_SETTINGS);
    }

    pub fn is_systray_enabled(&self) -> bool {
        (*self.configmanager_r)
            .borrow()
            .get_val_bool(&PropDef::SystrayEnable.to_string())
    }

    pub fn get_job_sender(&self) -> Sender<Job> {
        self.job_queue_sender.clone()
    }

    fn start_about_dialog(&self) {
        (*self.gui_updater).borrow().show_dialog(DIALOG_ABOUT);
    }

    ///  for key codes look at selec.rs                           gdk_sys::GDK_KEY_Escape => KeyCodes::Escape,
    fn process_key_press(&self, keycode: isize, _o_char: Option<char>) {
        let mut new_focus_by_tab = self.focus_by_tab.borrow().clone();
        let kc: KeyCodes = ui_select::from_gdk_sys(keycode);
        let subscription_id: isize = match (*self.feedsources_r)
            .borrow()
            .get_current_selected_subscription()
        {
            Some((subs_e, _)) => subs_e.subs_id,
            None => -1,
        };
        match kc {
            KeyCodes::Tab => new_focus_by_tab = self.focus_by_tab.borrow().next(),
            KeyCodes::ShiftTab => new_focus_by_tab = self.focus_by_tab.borrow().prev(),
            KeyCodes::Key_a => {
                trace!("GP:  key a subs_id:{} ", &subscription_id);
                if subscription_id > 0 {
                    (*self.feedsources_r).borrow().mark_as_read(subscription_id);
                }
            }
            KeyCodes::Key_s => {
                (*self.contentlist_r)
                    .borrow_mut()
                    .move_list_cursor(ListMoveCommand::PreviousUnreadMessage);
            }
            KeyCodes::Key_x => {
                (*self.contentlist_r)
                    .borrow_mut()
                    .move_list_cursor(ListMoveCommand::LaterUnreadMessage);
            }
            KeyCodes::Delete => {
                if *self.focus_by_tab.borrow() == FocusByTab::FocusMessages {
                    (*self.contentlist_r).borrow().keyboard_delete();
                } else {
                    debug!("delete key but unfocused");
                }
            }
            KeyCodes::Space => {
                if *self.focus_by_tab.borrow() == FocusByTab::FocusMessages {
                    (*self.contentlist_r).borrow().launch_browser_selected();
                } else {
                    debug!("space key but unfocused");
                }
            }
            _ => {
                // trace!("key-pressed: other {} {:?} {:?}", keycode, _o_char, kc);
            }
        }
        if new_focus_by_tab != *self.focus_by_tab.borrow() {
            self.focus_by_tab.replace(new_focus_by_tab);
            self.switch_focus_marker(true);
            self.addjob(Job::CheckFocusMarker(2));
            match *self.focus_by_tab.borrow() {
                FocusByTab::FocusSubscriptions => {
                    (*self.gui_updater)
                        .borrow()
                        .grab_focus(UIUpdaterMarkWidgetType::TreeView, TREEVIEW0);
                }
                FocusByTab::FocusMessages => {
                    (*self.gui_updater)
                        .borrow()
                        .grab_focus(UIUpdaterMarkWidgetType::TreeView, TREEVIEW1);
                }
                FocusByTab::FocusBrowser => {
                    (*self.gui_updater)
                        .borrow()
                        .grab_focus(UIUpdaterMarkWidgetType::WebView, 0);
                }
                _ => (),
            }
        }
    }

    fn switch_focus_marker(&self, marker_active: bool) {
        let mark = if marker_active { 1 } else { 2 };
        match *self.focus_by_tab.borrow() {
            FocusByTab::FocusSubscriptions => {
                (*self.gui_updater).borrow().widget_mark(
                    UIUpdaterMarkWidgetType::ScrolledWindow,
                    SCROLLEDWINDOW_0,
                    mark,
                );
            }
            FocusByTab::FocusMessages => {
                (*self.gui_updater).borrow().widget_mark(
                    UIUpdaterMarkWidgetType::ScrolledWindow,
                    SCROLLEDWINDOW_1,
                    mark,
                );
            }
            FocusByTab::FocusBrowser => {
                (*self.gui_updater).borrow().widget_mark(
                    UIUpdaterMarkWidgetType::Box,
                    BOX_CONTAINER_3_MARK,
                    mark,
                );
            }
            _ => (),
        }
    }

    pub fn startup_dialogs(&self) {
        let app_rcs_v = (*self.configmanager_r)
            .borrow()
            .get_sys_val(&PropDef::AppRcsVersion.to_string())
            .unwrap_or_else(|| "GP: no AppRcsVersion".to_string());
        let dd: Vec<AValue> = vec![AValue::ASTR(app_rcs_v)];
        (*self.gui_val_store)
            .write()
            .unwrap()
            .set_dialog_data(DIALOG_ABOUT, &dd);
        (*self.gui_updater).borrow().update_dialog(DIALOG_ABOUT);
    }

    pub fn add_handler<T: HandleSingleEvent + 'static>(&mut self, ev: &GuiEvents, handler: T) {
        self.event_handler_map
            .insert(std::mem::discriminant(ev), Box::new(handler));
    }

    // GuiProcessor
}

/// Statusbar  download processor display
/// https://www.w3schools.com/charsets/ref_utf_arrows.asp
// 2 => char::from_u32(0x25cc).unwrap(), // icon : dotted circle
// 4 => char::from_u32(0x2211).unwrap(), // ReadCounts : Sigma Sum sign, deprecated
pub fn dl_char_for_kind(kind: u8) -> char {
    let nc: char = match kind {
        1 => char::from_u32(0x2193).unwrap(), // feed-simple : arrow down
        2 => char::from_u32(0x2662).unwrap(), // icon : diamond sign
        3 => char::from_u32(0x21d3).unwrap(), // feed-comprehensive : double arrow
        4 => char::from_u32(0x26c1).unwrap(), // DatabaseCleanup : database icon
        5 => char::from_u32(0x21d3).unwrap(), // Drag Url eval : double arrow
        6 => char::from_u32(0x2191).unwrap(), // Launch Browser : arrow up
        _ => '_',
    };
    nc
}

impl Buildable for GuiProcessor {
    type Output = GuiProcessor;
    fn build(_conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
        let gp = GuiProcessor::new(_appcontext);
        gp.statusbar.borrow_mut().mem_usage_vmrss_bytes = -1;
        gp
    }
}

impl TimerReceiver for GuiProcessor {
    fn trigger(&self, event: &TimerEvent) {
        if *self.currently_minimized.borrow() {
            match event {
                TimerEvent::Timer1s => {
                    self.process_event();
                    self.process_jobs();
                }
                TimerEvent::Timer10s => {
                    self.statusbar.borrow_mut().update();
                }
                _ => (),
            }
        } else {
            match event {
                TimerEvent::Timer100ms => {
                    self.process_event();
                    self.process_jobs();
                    self.statusbar.borrow_mut().update();
                }
                TimerEvent::Timer1s => {
                    self.statusbar.borrow_mut().update_memory_stats();
                }
                TimerEvent::Startup => {
                    self.process_jobs();
                    self.startup_dialogs();
                }
                _ => (),
            }
        }
    }
}

impl StartupWithAppContext for GuiProcessor {
    fn startup(&mut self, ac: &AppContext) {
        let gp_r: Rc<RefCell<GuiProcessor>> = ac.get_rc::<GuiProcessor>().unwrap();
        {
            let mut t = (*self.timer_r).borrow_mut();
            t.register(&TimerEvent::Timer100ms, gp_r.clone(), false);
            t.register(&TimerEvent::Timer1s, gp_r.clone(), false);
            t.register(&TimerEvent::Timer10s, gp_r.clone(), false);
            t.register(&TimerEvent::Startup, gp_r, false);
            self.timer_sender = Some((*t).get_ctrl_sender());
        }
        self.addjob(Job::StartApplication);
        self.addjob(Job::NotifyConfigChanged);
        self.statusbar.borrow_mut().num_downloader_threads = (*self.downloader_r)
            .borrow()
            .get_config()
            .num_downloader_threads;
        if let Some(s) = (*self.configmanager_r)
            .borrow()
            .get_sys_val(ConfigManager::CONF_MODE_DEBUG)
        {
            if let Ok(b) = s.parse::<bool>() {
                self.statusbar.borrow_mut().mode_debug = b;
            }
        }
        self.add_handler(&GuiEvents::WinDelete, HandleWinDelete2 {});
        self.add_handler(
            &GuiEvents::DialogData(String::default(), Vec::default()),
            HandleDialogData {
                r_brow: self.browserpane_r.clone(),
                r_conf: self.configmanager_r.clone(),
                r_subm: self.subscriptionmove_r.clone(),
                r_stc: self.feedsources_r.clone(),
                r_subr: self.subscriptionrepo_r.clone(), //4
                r_dl: self.downloader_r.clone(),
                r_cl: self.contentlist_r.clone(), // 6
                r_gc: self.gui_context_r.clone(),
            },
        );
        self.add_handler(
            &GuiEvents::PanedMoved(0, 0),
            HandlePanedMoved(self.gui_context_r.clone(), self.configmanager_r.clone()),
        );
        self.add_handler(
            &GuiEvents::AppWasAlreadyRunning,
            HandleAppWasAlreadyRunning(),
        );
        self.add_handler(
            &GuiEvents::MenuActivate(String::default()),
            HandleMenuActivate(),
        );
        self.add_handler(
            &GuiEvents::TreeRowActivated(0, Vec::default(), 0),
            HandleTreeRowActivated(
                self.contentlist_r.clone(),
                self.feedsources_r.clone(),
                self.subscriptionmove_r.clone(),
            ),
        );
        self.add_handler(
            &GuiEvents::ListRowDoubleClicked(0, 0, 0),
            HandleListRowDoubleClicked(self.contentlist_r.clone()),
        );
        self.add_handler(
            &GuiEvents::ListCellClicked(0, 0, 0, 0),
            HandleListCellClicked(self.contentlist_r.clone()),
        );
        self.add_handler(
            &GuiEvents::WindowSizeChanged(0, 0),
            HandleWindowSizeChanged(self.configmanager_r.clone()),
        );
        self.add_handler(
            &GuiEvents::DialogEditData(String::default(), AValue::None),
            HandleDialogEditData(self.feedsources_r.clone()),
        );
        self.add_handler(
            &GuiEvents::TreeEvent(0, 0, String::default()),
            HandleTreeEvent(self.feedsources_r.clone()),
        );
        self.add_handler(
            &GuiEvents::TreeDragEvent(0, Vec::default(), Vec::default()),
            HandleTreeDragEvent(self.subscriptionmove_r.clone()),
        );
        self.add_handler(
            &GuiEvents::TreeExpanded(0, 0),
            HandleTreeExpanded(
                self.subscriptionrepo_r.clone(),
                self.subscriptionmove_r.clone(),
            ),
        );
        self.add_handler(
            &GuiEvents::TreeCollapsed(0, 0),
            HandleTreeCollapsed(
                self.subscriptionrepo_r.clone(),
                self.subscriptionmove_r.clone(),
            ),
        );
        self.add_handler(
            &GuiEvents::ToolBarButton(String::default()),
            HandleToolBarButton(self.feedsources_r.clone(), self.browserpane_r.clone()),
        );
        self.add_handler(
            &GuiEvents::ToolBarToggle(String::default(), false),
            HandleToolBarToggle(),
        );
        self.add_handler(
            &GuiEvents::ColumnWidth(0, 0),
            HandleColumnWidth(self.configmanager_r.clone()),
        );
        self.add_handler(
            &GuiEvents::ListSelected(0, Vec::default()),
            HandleListSelected(self.contentlist_r.clone()),
        );
        self.add_handler(
            &GuiEvents::ListSelectedAction(0, String::default(), Vec::default()),
            HandleListSelectedAction(self.contentlist_r.clone()),
        );
        self.add_handler(
            &GuiEvents::ListSortOrderChanged(0, 0, false),
            HandleListSortOrderChanged(self.contentlist_r.clone()),
        );
        self.add_handler(&GuiEvents::KeyPressed(0, None), HandleKeyPressed());
        self.add_handler(
            &GuiEvents::SearchEntryTextChanged(0, String::default()),
            HandleSearchEntryTextChanged(self.contentlist_r.clone()),
        );
        self.add_handler(
            &GuiEvents::WindowThemeChanged(String::default()),
            HandleWindowThemeChanged(self.gui_context_r.clone()),
        );
        self.add_handler(
            &GuiEvents::WindowIconified(false),
            HandleWindowIconified(
                self.feedsources_r.clone(),
                self.contentlist_r.clone(),
                self.gui_context_r.clone(),
            ),
        );
        self.add_handler(
            &GuiEvents::Indicator(String::default(), 0),
            HandleIndicator(),
        );
        self.add_handler(
            &GuiEvents::DragDropUrlReceived(String::default()),
            HandleDragDropUrlReceived(
                self.downloader_r.clone(),
                self.subscriptionmove_r.clone(),
                self.feedsources_r.clone(),
            ),
        );
        self.add_handler(
            &GuiEvents::BrowserEvent(BrowserEventType::default(), 0),
            HandleBrowserEvent(),
        );
    }
}

#[derive(Clone, Debug, PartialEq)]
enum FocusByTab {
    None,
    FocusSubscriptions,
    FocusMessages,
    FocusBrowser,
}

impl FocusByTab {
    fn next(&self) -> Self {
        match self {
            FocusByTab::None => FocusByTab::FocusSubscriptions,
            FocusByTab::FocusSubscriptions => FocusByTab::FocusMessages,
            FocusByTab::FocusMessages => FocusByTab::FocusBrowser,
            FocusByTab::FocusBrowser => FocusByTab::FocusSubscriptions,
        }
    }
    fn prev(&self) -> Self {
        match self {
            FocusByTab::None => FocusByTab::FocusBrowser,
            FocusByTab::FocusSubscriptions => FocusByTab::FocusBrowser,
            FocusByTab::FocusMessages => FocusByTab::FocusSubscriptions,
            FocusByTab::FocusBrowser => FocusByTab::FocusMessages,
        }
    }
}

// ---

struct HandleWinDelete2();
impl HandleSingleEvent for HandleWinDelete2 {
    fn handle(&self, _ev: GuiEvents, gp: &GuiProcessor) {
        gp.addjob(Job::StopApplication);
    }
}

struct HandleAppWasAlreadyRunning();
impl HandleSingleEvent for HandleAppWasAlreadyRunning {
    fn handle(&self, _ev: GuiEvents, gp: &GuiProcessor) {
        let _r = gp.timer_sender.as_ref().unwrap().send(TimerJob::Shutdown);
        gp.addjob(Job::StopApplication);
    }
}

struct HandleMenuActivate();
impl HandleSingleEvent for HandleMenuActivate {
    fn handle(&self, ev: GuiEvents, gp: &GuiProcessor) {
        if let GuiEvents::MenuActivate(ref s) = ev {
            match s.as_str() {
                "M_FILE_QUIT" => {
                    gp.addjob(Job::StopApplication);
                }
                "M_SETTINGS" => {
                    gp.start_settings_dialog();
                }
                "M_ABOUT" => {
                    gp.start_about_dialog();
                }
                "M_SHORT_HELP" => {
                    gp.browserpane_r.borrow().display_short_help();
                }
                _ => warn!("Menu Unprocessed:{:?} ", s),
            }
        }
    }
}

struct HandleTreeRowActivated(
    Rc<RefCell<dyn IContentList>>,
    Rc<RefCell<dyn ISourceTreeController>>,
    Rc<RefCell<dyn ISubscriptionMove>>, // 2
);
impl HandleSingleEvent for HandleTreeRowActivated {
    fn handle(&self, ev: GuiEvents, gp: &GuiProcessor) {
        if let GuiEvents::TreeRowActivated(_tree_idx, ref path_u16, subs_id) = ev {
            // trace!("TreeRowActivated : {:?} {:?} ", subs_id, path_u16);
            let statemap_rc = (*self.2).borrow().get_state_map();
            if let Some(subs_map) = statemap_rc.borrow().get_state(subs_id as isize) {
                if let Some(tp) = subs_map.tree_path {
                    if tp != *path_u16 {
                        debug!(
                            "TreeRowActivated : {:?}   unequal STATE_PATH: {:?} ",
                            subs_id,
                            tp
                        );
                        return;
                    }
                }
            }
            (*self.1)
                .borrow_mut()
                .set_ctx_subscription(subs_id as isize);
            (*self.0).borrow().update_message_list_(subs_id as isize);
            gp.focus_by_tab.replace(FocusByTab::FocusSubscriptions);
        }
    }
}

struct HandleListRowDoubleClicked(Rc<RefCell<dyn IContentList>>);
impl HandleSingleEvent for HandleListRowDoubleClicked {
    fn handle(&self, ev: GuiEvents, gp: &GuiProcessor) {
        if let GuiEvents::ListRowDoubleClicked(_list_idx, _list_position, fc_repo_id) = ev {
            gp.focus_by_tab.replace(FocusByTab::FocusMessages);
            (*self.0).borrow().launch_browser_single(vec![fc_repo_id]);
        }
    }
}

struct HandleListCellClicked(Rc<RefCell<dyn IContentList>>);
impl HandleSingleEvent for HandleListCellClicked {
    fn handle(&self, ev: GuiEvents, gp: &GuiProcessor) {
        if let GuiEvents::ListCellClicked(_list_idx, list_position, sort_col_nr, msg_id) = ev {
            gp.focus_by_tab.replace(FocusByTab::FocusMessages);
            if sort_col_nr == LIST0_COL_ISREAD && msg_id >= 0 {
                (*self.0)
                    .borrow()
                    .toggle_feed_item_read(msg_id as isize, list_position);
            } else if sort_col_nr == LIST0_COL_FAVICON && msg_id >= 0 {
                (*self.0)
                    .borrow()
                    .toggle_favorite(msg_id as isize, list_position, None);
            } else {
                warn!("ListCellClicked msg{}  col{} ", msg_id, sort_col_nr);
            }
        }
    }
}

struct HandlePanedMoved(Rc<RefCell<GuiContext>>, Rc<RefCell<ConfigManager>>);
impl HandleSingleEvent for HandlePanedMoved {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::PanedMoved(pane_id, pos) = ev {
            match pane_id {
                0 => {
                    if pos < TREE_PANE1_MIN_WIDTH {
                        let u_a_r = (*self.0).borrow().get_updater_adapter();

                        (*u_a_r)
                            .borrow()
                            .update_paned_pos(PANED_1_LEFT, TREE_PANE1_MIN_WIDTH);
                    } else {
                        (*self.1).borrow_mut().store_gui_pane1_pos(pos);
                    }
                }
                1 => (*self.1).borrow_mut().store_gui_pane2_pos(pos),
                _ => {}
            }
        }
    }
}

struct HandleWindowSizeChanged(Rc<RefCell<ConfigManager>>);
impl HandleSingleEvent for HandleWindowSizeChanged {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::WindowSizeChanged(width, height) = ev {
            (*self.0).borrow_mut().store_window_size(width, height);
        }
    }
}

struct HandleDialogData {
    r_brow: Rc<RefCell<dyn IBrowserPane>>, // 0
    r_conf: Rc<RefCell<ConfigManager>>,
    r_subm: Rc<RefCell<dyn ISubscriptionMove>>, // 2
    r_stc: Rc<RefCell<dyn ISourceTreeController>>,
    r_subr: Rc<RefCell<dyn ISubscriptionRepo>>, // 4
    r_dl: Rc<RefCell<dyn IDownloader>>,
    r_cl: Rc<RefCell<dyn IContentList>>, // 6
    r_gc: Rc<RefCell<GuiContext>>,
}
impl HandleSingleEvent for HandleDialogData {
    fn handle(&self, ev: GuiEvents, gp: &GuiProcessor) {
        if let GuiEvents::DialogData(ref ident, ref payload) = ev {
            match ident.as_str() {
                "new-folder" => {
                    if let Some(AValue::ASTR(s)) = payload.get(0) {
                        (*self.r_subm).borrow_mut().add_new_folder(s.to_string());
                    }
                }
                "new-feedsource" => {
                    if payload.len() < 2 {
                        error!("new-feedsource, too few data ");
                    } else if let (Some(AValue::ASTR(ref s0)), Some(AValue::ASTR(ref s1))) =
                        (payload.get(0), payload.get(1))
                    {
                        let new_id = self
                            .r_subm
                            .borrow_mut()
                            .add_new_subscription(s0.clone(), s1.clone());
                        if new_id > 0 {
                            self.r_stc
                                .borrow_mut()
                                .addjob(SJob::ScheduleUpdateFeed(new_id));
                        }
                    }
                }
                "import-opml" => {
                    if let Some(AValue::ASTR(ref s)) = payload.get(0) {
                        self.r_subm.borrow_mut().import_opml(s.to_string());
                    }
                }
                "export-opml" => {
                    if let Some(AValue::ASTR(ref s)) = payload.get(0) {
                        let mut opmlreader = OpmlReader::new(self.r_subr.clone());
                        opmlreader.transfer_from_db();
                        match opmlreader.write_to_file(s.to_string()) {
                            Ok(()) => {
                                debug!("Writing {} success ", s);
                            }
                            Err(e) => {
                                warn!("Writing {} : {:?}", s, e);
                            }
                        }
                    }
                }
                "feedsource-delete" => {
                    self.r_subm.borrow_mut().move_subscription_to_trash();
                }
                "subscription-edit-ok" => {
                    self.r_stc.borrow_mut().end_feedsource_edit_dialog(payload);
                }
                "folder-edit" => {
                    self.r_stc.borrow_mut().end_feedsource_edit_dialog(payload);
                }
                "settings" => {
                    self.r_stc
                        .borrow_mut()
                        .set_conf_load_on_start(payload.get(0).unwrap().boo());
                    self.r_stc
                        .borrow_mut()
                        .set_conf_fetch_interval(payload.get(1).unwrap().int().unwrap());
                    self.r_stc
                        .borrow_mut()
                        .set_conf_fetch_interval_unit(payload.get(2).unwrap().int().unwrap());
                    self.r_dl
                        .borrow_mut()
                        .set_conf_num_threads(payload.get(3).unwrap().int().unwrap() as u8);
                    self.r_cl
                        .borrow_mut()
                        .set_conf_focus_policy(payload.get(4).unwrap().int().unwrap() as u8);
                    self.r_stc
                        .borrow_mut() // 5 : DisplayCountOfAllFeeds
                        .set_conf_display_feedcount_all(payload.get(5).unwrap().boo());
                    self.r_cl
                        .borrow_mut()
                        .set_conf_msg_keep_count(payload.get(6).unwrap().int().unwrap());
                    (*self.r_gc)
                        .borrow() // 7 : ManualFontSizeEnable
                        .set_conf_fontsize_manual_enable(payload.get(7).unwrap().boo());
                    (*self.r_gc)
                        .borrow() // 8 : ManualFontSize
                        .set_conf_fontsize_manual(payload.get(8).unwrap().int().unwrap());
                    (*self.r_brow)
                        .borrow_mut() // 9 : browser bg
                        .set_conf_browser_bg(payload.get(9).unwrap().uint().unwrap());
                    self.r_conf.borrow().set_val(
                        &PropDef::BrowserClearCache.to_string(),
                        payload.get(10).unwrap().boo().to_string(), // 10 : browser cache cleanup
                    );
                    self.r_conf.borrow().set_val(
                        contentdownloader::CONF_DATABASES_CLEANUP, // 11 : DB cleanup
                        payload.get(11).unwrap().boo().to_string(),
                    );

                    if let Some(systray_e) = payload.get(12) {
                        self.r_conf.borrow().set_val(
                            &PropDef::SystrayEnable.to_string(),
                            systray_e.boo().to_string(), // 12 : enable systray
                        );
                    }
                    gp.addjob(Job::NotifyConfigChanged);
                }
                _ => {
                    warn!("other DialogData: {:?}  {:?} ", &ident, payload);
                }
            }
        }
    }
}

struct HandleDialogEditData(Rc<RefCell<dyn ISourceTreeController>>);
impl HandleSingleEvent for HandleDialogEditData {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::DialogEditData(ref ident, ref payload) = ev {
            match ident.as_str() {
                "feedsource-edit" => {
                    if let Some(edit_url) = payload.str() {
                        (*self.0).borrow_mut().newsource_dialog_edit(edit_url);
                    }
                }
                _ => {
                    warn!(" other DialogEditData  {:?} {:?}", &ident, payload);
                }
            }
        }
    }
}

struct HandleTreeEvent(Rc<RefCell<dyn ISourceTreeController>>);
impl HandleSingleEvent for HandleTreeEvent {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::TreeEvent(_tree_nr, src_repo_id, ref command) = ev {
            match command.as_str() {
                "feedsource-delete-dialog" => {
                    (*self.0)
                        .borrow_mut()
                        .start_delete_dialog(src_repo_id as isize);
                }
                "feedsource-update" => {
                    self.0
                        .borrow_mut()
                        .mark_schedule_fetch(src_repo_id as isize);
                }
                "feedsource-edit-dialog" => {
                    (*self.0)
                        .borrow_mut()
                        .start_feedsource_edit_dialog(src_repo_id as isize);
                }
                "feedsource-mark-as-read" => {
                    (*self.0).borrow_mut().mark_as_read(src_repo_id as isize);
                }
                "new-folder-dialog" => {
                    (*self.0)
                        .borrow_mut()
                        .start_new_fol_sub_dialog(src_repo_id as isize, DIALOG_NEW_FOLDER);
                }
                "new-subscription-dialog" => {
                    (*self.0)
                        .borrow_mut()
                        .start_new_fol_sub_dialog(src_repo_id as isize, DIALOG_NEW_SUBSCRIPTION);
                }
                _ => {
                    warn!("unknown command for TreeEvent   {}", command);
                }
            }
        }
    }
}

struct HandleTreeDragEvent(Rc<RefCell<dyn ISubscriptionMove>>);
impl HandleSingleEvent for HandleTreeDragEvent {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::TreeDragEvent(_tree_nr, ref from_path, ref to_path) = ev {
            let _success =
                self.0
                    .borrow()
                    .on_subscription_drag(_tree_nr, from_path.clone(), to_path.clone());
        }
    }
}

struct HandleTreeExpanded(
    Rc<RefCell<dyn ISubscriptionRepo>>,
    Rc<RefCell<dyn ISubscriptionMove>>,
);
impl HandleSingleEvent for HandleTreeExpanded {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::TreeExpanded(_idx, repo_id) = ev {
            let statemap_rc = (*self.1).borrow().get_state_map();
            (*statemap_rc).borrow_mut().set_status(
                &[repo_id as isize],
                StatusMask::IsExpandedCopy,
                true,
            );
            (*self.0)
                .borrow()
                .update_expanded([repo_id as isize].to_vec(), true);
        }
    }
}

struct HandleTreeCollapsed(
    Rc<RefCell<dyn ISubscriptionRepo>>,
    Rc<RefCell<dyn ISubscriptionMove>>,
);
impl HandleSingleEvent for HandleTreeCollapsed {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::TreeCollapsed(_idx, repo_id) = ev {
            let statemap_rc = (*self.1).borrow().get_state_map();
            (*statemap_rc).borrow_mut().set_status(
                &[repo_id as isize],
                StatusMask::IsExpandedCopy,
                false,
            );
            (*self.0)
                .borrow()
                .update_expanded(vec![repo_id as isize], false);
        }
    }
}

struct HandleToolBarButton(
    Rc<RefCell<dyn ISourceTreeController>>,
    Rc<RefCell<dyn IBrowserPane>>,
);
impl HandleSingleEvent for HandleToolBarButton {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::ToolBarButton(ref id) = ev {
            match id.as_str() {
                "reload-subscriptions-all" => {
                    let o_c = (*self.0).borrow().get_current_selected_subscription();
                    debug!("RELOAD-ALL {:?} ", o_c);
                    match o_c {
                        Some((subs_e, _)) => {
                            self.0
                                .borrow_mut()
                                .addjob(SJob::ScheduleUpdateFeed(subs_e.subs_id));
                        }
                        None => {
                            trace!("no current id found, cannot update ")
                        }
                    };
                }
                "browser-zoom-in" => {
                    self.1.borrow().set_browser_zoom(BrowserZoomCommand::ZoomIn);
                }
                "browser-zoom-out" => {
                    self.1
                        .borrow()
                        .set_browser_zoom(BrowserZoomCommand::ZoomOut);
                }
                "browser-zoom-default" => {
                    self.1
                        .borrow()
                        .set_browser_zoom(BrowserZoomCommand::ZoomDefault);
                }
                _ => {
                    warn!("unknown ToolBarButton {} ", id);
                }
            }
        }
    }
}

struct HandleToolBarToggle();
impl HandleSingleEvent for HandleToolBarToggle {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::ToolBarToggle(ref id, active) = ev {
            match id.as_str() {
                "special1" => {
                    debug!(" ToolBarToggle special1 {} {} ", id, active);
                }
                _ => {
                    warn!("unknown ToolBarToggle {} ", id);
                }
            }
        }
    }
}

struct HandleColumnWidth(Rc<RefCell<ConfigManager>>);
impl HandleSingleEvent for HandleColumnWidth {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::ColumnWidth(col_nr, width) = ev {
            self.0.borrow_mut().store_column_width(col_nr, width);
        }
    }
}

struct HandleListSelected(Rc<RefCell<dyn IContentList>>);
impl HandleSingleEvent for HandleListSelected {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::ListSelected(_list_idx, ref selected_list) = ev {
            self.0
                .borrow()
                .set_selected_content_ids(selected_list.clone());
        }
    }
}

struct HandleListSelectedAction(Rc<RefCell<dyn IContentList>>);
impl HandleSingleEvent for HandleListSelectedAction {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::ListSelectedAction(list_idx, ref action, ref repoid_list_pos) = ev {
            if list_idx == 0 {
                self.0
                    .borrow()
                    .process_list_action(action.clone(), repoid_list_pos.clone());
            }
        }
    }
}

struct HandleListSortOrderChanged(Rc<RefCell<dyn IContentList>>);
impl HandleSingleEvent for HandleListSortOrderChanged {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::ListSortOrderChanged(list_idx, col_id, ascending) = ev {
            if list_idx == 0 {
                self.0.borrow_mut().set_sort_order(col_id, ascending);
            }
        }
    }
}

struct HandleKeyPressed();
impl HandleSingleEvent for HandleKeyPressed {
    fn handle(&self, ev: GuiEvents, gp: &GuiProcessor) {
        if let GuiEvents::KeyPressed(keycode, o_char) = ev {
            gp.process_key_press(keycode, o_char);
        }
    }
}

struct HandleSearchEntryTextChanged(Rc<RefCell<dyn IContentList>>);
impl HandleSingleEvent for HandleSearchEntryTextChanged {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::SearchEntryTextChanged(_idx, ref newtext) = ev {
            self.0.borrow_mut().set_messages_filter(newtext);
        }
    }
}

struct HandleWindowThemeChanged(Rc<RefCell<GuiContext>>);
impl HandleSingleEvent for HandleWindowThemeChanged {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::WindowThemeChanged(ref theme_name) = ev {
            self.0.borrow().set_theme_name(theme_name);
        }
    }
}

struct HandleWindowIconified(
    Rc<RefCell<dyn ISourceTreeController>>,
    Rc<RefCell<dyn IContentList>>, // 6
    Rc<RefCell<GuiContext>>,
);
impl HandleSingleEvent for HandleWindowIconified {
    fn handle(&self, ev: GuiEvents, gp: &GuiProcessor) {
        if let GuiEvents::WindowIconified(is_minimized) = ev {
            gp.currently_minimized.replace(is_minimized);
            self.0.borrow_mut().memory_conserve(is_minimized);
            (*self.1).borrow_mut().memory_conserve(is_minimized);
            (*self.2)
                .borrow()
                .get_values_adapter()
                .write()
                .unwrap()
                .memory_conserve(is_minimized);
            (*self.2)
                .borrow()
                .get_updater_adapter()
                .borrow()
                .memory_conserve(is_minimized);
        }
    }
}

struct HandleIndicator();
impl HandleSingleEvent for HandleIndicator {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::Indicator(ref cmd, _gtktime) = ev {
            debug!(" indicator event {}", cmd)
        }
    }
}

struct HandleDragDropUrlReceived(
    Rc<RefCell<dyn IDownloader>>,
    Rc<RefCell<dyn ISubscriptionMove>>,
    Rc<RefCell<dyn ISourceTreeController>>,
);
impl HandleSingleEvent for HandleDragDropUrlReceived {
    fn handle(&self, ev: GuiEvents, _gp: &GuiProcessor) {
        if let GuiEvents::DragDropUrlReceived(ref url) = ev {
            if let Some((subs_e, _nf_childs)) = self.2.borrow().get_current_selected_subscription()
            {
                self.1
                    .borrow_mut()
                    .set_new_folder_parent(subs_e.parent_subs_id);
            } else {
                debug!("Drag, having no  parent folder! ");
            }
            self.0.borrow().browser_drag_request(url);
        }
    }
}

struct HandleBrowserEvent();
impl HandleSingleEvent for HandleBrowserEvent {
    fn handle(&self, ev: GuiEvents, gp: &GuiProcessor) {
        if let GuiEvents::BrowserEvent(ref ev_type, value) = ev {
            if ev_type == &BrowserEventType::LoadingProgress {
                gp.statusbar.borrow_mut().browser_loading_progress = value as u8;
            }
        }
    }
}
