use crate::config::configmanager::ConfigManager;
use crate::controller::browserpane;
use crate::controller::browserpane::BrowserZoomCommand;
use crate::controller::browserpane::IBrowserPane;
use crate::controller::contentdownloader;
use crate::controller::contentdownloader::IDownloader;
use crate::controller::contentlist::FeedContents;
use crate::controller::contentlist::IFeedContents;
use crate::controller::contentlist::ListMoveCommand;
use crate::controller::sourcetree::ISourceTreeController;
use crate::controller::sourcetree::SJob;
use crate::controller::sourcetree::SourceTreeController;
use crate::db::errors_repo::ErrorRepo;
use crate::db::icon_repo::IconEntry;
use crate::db::icon_repo::IconRepo;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::db::subscription_state::SubsMapEntry;
use crate::opml::opmlreader::OpmlReader;
use crate::timer::ITimer;
use crate::timer::Timer;
use crate::timer::TimerJob;
use crate::ui_select::gui_context::GuiContext;
use crate::ui_select::select::ui_select;
use crate::util::string_escape_url;
use crate::util::timestamp_now;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use context::TimerEvent;
use context::TimerReceiver;
use flume::Receiver;
use flume::Sender;
use gui_layer::abstract_ui::AValue;
use gui_layer::abstract_ui::GuiEvents;
use gui_layer::abstract_ui::GuiRunner;
use gui_layer::abstract_ui::KeyCodes;
use gui_layer::abstract_ui::UIAdapterValueStoreType;
use gui_layer::abstract_ui::UIUpdaterAdapter;
use gui_layer::abstract_ui::UIUpdaterMarkWidgetType;
use gui_layer::gui_values::PropDef;
use resources::gen_icons;
use resources::id::DIALOG_ABOUT;
use resources::id::*;
use resources::parameter::DOWNLOADER_MAX_NUM_THREADS;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
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
    feedsources_r: Rc<RefCell<dyn ISourceTreeController>>,
    feedcontents_r: Rc<RefCell<dyn IFeedContents>>,
    downloader_r: Rc<RefCell<dyn IDownloader>>,
    gui_context_r: Rc<RefCell<GuiContext>>,
    browserpane_r: Rc<RefCell<dyn IBrowserPane>>,
    subscriptionrepo_r: Rc<RefCell<dyn ISubscriptionRepo>>,
    iconrepo_r: Rc<RefCell<IconRepo>>,
    statusbar_items: StatusBarItems,
    focus_by_tab: FocusByTab,
    erro_repo_r: Rc<RefCell<ErrorRepo>>,
    currently_minimized: bool,
}

impl GuiProcessor {
    pub fn new(ac: &AppContext) -> Self {
        let (q_s, q_r) = flume::bounded::<Job>(JOBQUEUE_SIZE);
        let guicontex_r = (*ac).get_rc::<GuiContext>().unwrap();
        let u_a = (*guicontex_r).borrow().get_updater_adapter();
        let v_s_a = (*guicontex_r).borrow().get_values_adapter();
        let guirunner = (*guicontex_r).borrow().get_gui_runner();
        let dl_r = (*ac).get_rc::<contentdownloader::Downloader>().unwrap();
        let err_rep = (*ac).get_rc::<ErrorRepo>().unwrap();
        GuiProcessor {
            subscriptionrepo_r: (*ac).get_rc::<SubscriptionRepo>().unwrap(),
            configmanager_r: (*ac).get_rc::<ConfigManager>().unwrap(),
            feedsources_r: (*ac).get_rc::<SourceTreeController>().unwrap(),
            feedcontents_r: (*ac).get_rc::<FeedContents>().unwrap(), // YY
            iconrepo_r: (*ac).get_rc::<IconRepo>().unwrap(),
            timer_r: (*ac).get_rc::<Timer>().unwrap(),
            job_queue_sender: q_s,
            job_queue_receiver: q_r,
            timer_sender: None,
            gui_updater: u_a,
            gui_val_store: v_s_a,
            gui_runner: guirunner,
            downloader_r: dl_r,
            gui_context_r: guicontex_r,
            browserpane_r: (*ac).get_rc::<browserpane::BrowserPane>().unwrap(),
            statusbar_items: StatusBarItems::default(),
            focus_by_tab: FocusByTab::None,
            erro_repo_r: err_rep,
            currently_minimized: false,
        }
    }

    pub fn process_event(&mut self) {
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
            let now = Instant::now();
            match ev {
                GuiEvents::None => {}
                GuiEvents::WinDelete => {
                    self.addjob(Job::StopApplication);
                }
                GuiEvents::AppWasAlreadyRunning => {
                    let _r = self.timer_sender.as_ref().unwrap().send(TimerJob::Shutdown);
                    self.addjob(Job::StopApplication);
                }
                GuiEvents::MenuActivate(ref s) => match s.as_str() {
                    "M_FILE_QUIT" => {
                        debug!("sending: Job::StopApplication");
                        self.addjob(Job::StopApplication);
                    }
                    "M_SETTINGS" => {
                        self.start_settings_dialog();
                    }
                    "M_ABOUT" => {
                        self.start_about_dialog();
                    }
                    "M_SHORT_HELP" => {
                        (*self.browserpane_r).borrow().display_short_help();
                    }
                    _ => warn!("Menu Unprocessed:{:?} ", s),
                },
                GuiEvents::ButtonClicked(ref b) => match b.as_str() {
                    "button1" => {
                        info!("ButtonClicked: button1");
                    }
                    _ => debug!("GP2 Button:  {:?}", b),
                },
                GuiEvents::TreeRowActivated(_tree_idx, ref _path, subs_id) => {
                    (*self.feedsources_r) // set it first into sources, we need that at contents for the focus
                        .borrow_mut()
                        .set_selected_feedsource(subs_id as isize);
                    // trace!(                        "GP:TreeRowActivated{} #q:{} {:?} ",                        subs_id,                        (*self.downloader_r).borrow().get_queue_size(),                        _path                    );
                    (*self.feedcontents_r)
                        .borrow()
                        .update_message_list(subs_id as isize);
                    self.focus_by_tab = FocusByTab::FocusSubscriptions;
                }
                GuiEvents::ListRowActivated(_list_idx, list_position, msg_id) => {
                    self.focus_by_tab = FocusByTab::FocusMessages;
                    list_row_activated_map.insert(msg_id, list_position);
                }
                GuiEvents::ListRowDoubleClicked(_list_idx, _list_position, fc_repo_id) => {
                    self.focus_by_tab = FocusByTab::FocusMessages;
                    (*self.feedcontents_r)
                        .borrow()
                        .start_web_browser(vec![fc_repo_id]);
                }
                GuiEvents::ListCellClicked(_list_idx, list_position, sort_col_nr, msg_id) => {
                    self.focus_by_tab = FocusByTab::FocusMessages;
                    if sort_col_nr == LIST0_COL_ISREAD && msg_id >= 0 {
                        (*self.feedcontents_r)
                            .borrow()
                            .toggle_feed_item_read(msg_id as isize, list_position);
                    }
                }
                GuiEvents::PanedMoved(pane_id, pos) => match pane_id {
                    0 => {
                        if pos < TREE_PANE1_MIN_WIDTH {
                            (*self.gui_updater)
                                .borrow()
                                .update_paned_pos(PANED_1_LEFT, TREE_PANE1_MIN_WIDTH);
                        } else {
                            (*(self.configmanager_r.borrow_mut())).store_gui_pane1_pos(pos);
                        }
                    }
                    1 => (*(self.configmanager_r.borrow_mut())).store_gui_pane2_pos(pos),
                    _ => {}
                },
                GuiEvents::WindowSizeChanged(width, height) => {
                    (*(self.configmanager_r.borrow_mut())).store_window_size(width, height);
                }
                GuiEvents::DialogData(ref ident, ref payload) => {
                    self.process_dialogdata(ident.clone(), payload.clone());
                }
                GuiEvents::DialogEditData(ref ident, ref payload) => match ident.as_str() {
                    "feedsource-edit" => {
                        if let Some(edit_url) = payload.str() {
                            (*self.feedsources_r)
                                .borrow_mut()
                                .newsource_dialog_edit(edit_url);
                        }
                    }
                    _ => {
                        warn!(" other DialogEditData  {:?} {:?}", &ident, payload);
                    }
                },
                GuiEvents::TreeEvent(_tree_nr, src_repo_id, ref command) => {
                    match command.as_str() {
                        "feedsource-delete-dialog" => {
                            debug!("feedsource-delete-dialog {:?}", src_repo_id);
                            (*self.feedsources_r)
                                .borrow_mut()
                                .start_delete_dialog(src_repo_id as isize);
                        }
                        "feedsource-update" => {
                            self.feedsources_r
                                .borrow_mut()
                                .mark_schedule_fetch(src_repo_id as isize);
                        }
                        "feedsource-edit-dialog" => {
                            (*self.feedsources_r)
                                .borrow_mut()
                                .start_feedsource_edit_dialog(src_repo_id as isize);
                        }
                        "feedsource-mark-as-read" => {
                            debug!("GP command: {:?}", command);
                            (*self.feedsources_r)
                                .borrow_mut()
                                .mark_as_read(src_repo_id as isize);
                        }
                        "new-folder-dialog" => {
                            (*self.feedsources_r)
                                .borrow_mut()
                                .start_new_fol_sub_dialog(src_repo_id as isize, DIALOG_NEW_FOLDER);
                        }
                        "new-subscription-dialog" => {
                            (*self.feedsources_r).borrow_mut().start_new_fol_sub_dialog(
                                src_repo_id as isize,
                                DIALOG_NEW_FEED_SOURCE,
                            );
                        }
                        _ => {
                            warn!("unknown command for TreeEvent   {}", command);
                        }
                    }
                }
                GuiEvents::TreeDragEvent(_tree_nr, ref from_path, ref to_path) => {
                    let _success = self.feedsources_r.borrow().on_fs_drag(
                        _tree_nr,
                        from_path.clone(),
                        to_path.clone(),
                    );
                }
                GuiEvents::TreeExpanded(_idx, repo_id) => {
                    self.feedsources_r
                        .borrow()
                        .set_tree_expanded(repo_id as isize, true);
                }
                GuiEvents::TreeCollapsed(_idx, repo_id) => {
                    self.feedsources_r
                        .borrow()
                        .set_tree_expanded(repo_id as isize, false);
                }
                GuiEvents::ToolBarButton(ref id) => match id.as_str() {
                    "reload-feeds-all" => {
                        self.feedsources_r
                            .borrow_mut()
                            .addjob(SJob::ScheduleFetchAllFeeds);
                    }
                    "browser-zoom-in" => {
                        (*self.browserpane_r)
                            .borrow()
                            .set_browser_zoom(BrowserZoomCommand::ZoomIn);
                    }
                    "browser-zoom-out" => {
                        (*self.browserpane_r)
                            .borrow()
                            .set_browser_zoom(BrowserZoomCommand::ZoomOut);
                    }
                    "browser-zoom-default" => {
                        (*self.browserpane_r)
                            .borrow()
                            .set_browser_zoom(BrowserZoomCommand::ZoomDefault);
                    }
                    "toolbutton-troubleshoot1" => {
                        debug!("toolbutton-troubleshoot1");
                    }
                    _ => {
                        warn!("unknown ToolBarButton {} ", id);
                    }
                },
                GuiEvents::ToolBarToggle(ref id, active) => match id.as_str() {
                    "special1" => {
                        let mark = if active { 1 } else { 2 };
                        debug!(" ToolBarToggle {} {} {}", id, active, mark);
                        (*self.gui_updater).borrow().widget_mark(
                            UIUpdaterMarkWidgetType::ScrolledWindow,
                            SCROLLEDWINDOW_0,
                            mark,
                        );

                        (*self.gui_updater).borrow().widget_mark(
                            UIUpdaterMarkWidgetType::Box,
                            BOX_CONTAINER_3_MARK,
                            mark,
                        );
                        (*self.gui_updater).borrow().widget_mark(
                            UIUpdaterMarkWidgetType::ScrolledWindow,
                            SCROLLEDWINDOW_1,
                            mark,
                        );
                    }
                    _ => {
                        warn!("unknown ToolBarToggle {} ", id);
                    }
                },

                GuiEvents::ColumnWidth(col_nr, width) => {
                    (*(self.configmanager_r.borrow_mut())).store_column_width(col_nr, width);
                }
                GuiEvents::ListSelected(_list_idx, ref selected_list) => {
                    (*self.feedcontents_r)
                        .borrow()
                        .set_selected_content_ids(selected_list.clone());
                }
                GuiEvents::ListSelectedAction(list_idx, ref action, ref repoid_list_pos) => {
                    if list_idx == 0 {
                        (*self.feedcontents_r)
                            .borrow()
                            .process_list_action(action.clone(), repoid_list_pos.clone());
                    }
                }
                GuiEvents::ListSortOrderChanged(list_idx, col_id, ascending) => {
                    if list_idx == 0 {
                        (*self.feedcontents_r)
                            .borrow_mut()
                            .set_sort_order(col_id, ascending);
                    }
                }
                GuiEvents::KeyPressed(keycode, o_char) => {
                    self.process_key_press(keycode, o_char);
                }
                GuiEvents::SearchEntryTextChanged(_idx, ref newtext) => {
                    (*self.feedcontents_r)
                        .borrow_mut()
                        .set_messages_filter(newtext);
                }
                GuiEvents::WindowThemeChanged(ref theme_name) => {
                    (*self.gui_context_r).borrow().set_theme_name(theme_name);
                }
                GuiEvents::WindowIconified(is_minimized) => {
                    self.currently_minimized = is_minimized;
                    debug!("EV -> IsMini:   {} ", self.currently_minimized);
                    (*self.feedsources_r)
                        .borrow_mut()
                        .memory_conserve(is_minimized);
                    (*self.feedcontents_r)
                        .borrow_mut()
                        .memory_conserve(is_minimized);
                    (*self.gui_val_store)
                        .write()
                        .unwrap()
                        .memory_conserve(is_minimized);
                    (*self.gui_updater).borrow().memory_conserve(is_minimized);
                }

                GuiEvents::Indicator(ref cmd, gtktime) => match cmd.as_str() {
                    "app-quit" => {
                        self.addjob(Job::StopApplication);
                    }
                    "show-window" => {
                        // debug!(                            "Indicator -> show-window!  cur-min {}  time:{}",                            self.currently_minimized, gtktime                        );
                        (*self.gui_updater)
                            .borrow()
                            .update_window_minimized(!self.currently_minimized, gtktime);
                    }
                    _ => {
                        warn!("unknown indicator event");
                    }
                },
                _ => {
                    warn!("other GuiEvents: {:?}", &ev);
                }
            }

            let elapsed_m = now.elapsed().as_millis();
            if elapsed_m > 100 {
                debug!("EV  {:?}   took {:?}", &ev, elapsed_m);
            }
        }
        if !list_row_activated_map.is_empty() {
            (*self.feedcontents_r)
                .borrow()
                .process_list_row_activated(&list_row_activated_map);
        }
    }

    /// is run by  the timer
    pub fn process_jobs(&mut self) {
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
                    (*self.gui_updater)
                        .borrow()
                        .update_systray_indicator(self.is_systray_enabled());
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
                    //                    debug!(                        "GP: SwitchContentList {}   => update_feed_list_contents ",                        feed_source_id                    );
                    (*self.feedcontents_r)
                        .borrow()
                        .update_message_list(feed_source_id);
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
                    (*self.feedcontents_r).borrow_mut().notify_config_update();
                    (*self.feedsources_r).borrow_mut().notify_config_update();
                    (*self.gui_updater)
                        .borrow()
                        .update_systray_indicator(self.is_systray_enabled());
                }
                Job::DownloaderJobStarted(threadnr, kind) => {
                    self.statusbar_items.downloader_kind_new[threadnr as usize] = kind;
                }
                Job::DownloaderJobFinished(subs_id, threadnr, _kind, elapsed_ms, description) => {
                    if elapsed_ms > 5000 && subs_id > 0 {
                        //  trace!(                            "DL: {} {} took {} ms {}    ",                           threadnr,                            _kind,                            elapsed_ms,                            description                        );
                        (*self.erro_repo_r).borrow().add_error(
                            subs_id,
                            elapsed_ms as isize,
                            String::default(),
                            description,
                        );
                    }

                    self.statusbar_items.downloader_kind_new[threadnr as usize] = 0;
                }
                Job::CheckFocusMarker(num) => {
                    if num > 0 {
                        self.addjob(Job::CheckFocusMarker(num - 1))
                    } else {
                        self.switch_focus_marker(false);
                    }
                }
                _ => {
                    warn!("other job! {:?}", &job);
                }
            }
            // debug!("GP:  job {:?}   done ", &job);
            let elapsedms = now.elapsed().as_millis();
            if elapsedms > 100 {
                warn!("JOB {:?} took {:?}ms", &job2, elapsedms);
            }
        }
    }

    pub fn process_dialogdata(&mut self, ident: String, payload: Vec<AValue>) {
        match ident.as_str() {
            "new-folder" => {
                if let Some(AValue::ASTR(s)) = payload.get(0) {
                    (*self.feedsources_r)
                        .borrow_mut()
                        .add_new_folder(s.to_string());
                    self.feedsources_r
                        .borrow_mut()
                        .addjob(SJob::FillSourcesTree);
                }
            }
            "new-feedsource" => {
                if payload.len() < 2 {
                    error!("new-feedsource, too few data ");
                } else if let (Some(AValue::ASTR(ref s0)), Some(AValue::ASTR(ref s1))) =
                    (payload.get(0), payload.get(1))
                {
                    let new_id = self
                        .feedsources_r
                        .borrow_mut()
                        .add_new_subscription(s0.clone(), s1.clone());
                    self.feedsources_r
                        .borrow_mut()
                        .addjob(SJob::FillSourcesTree);
                    self.feedsources_r
                        .borrow_mut()
                        .addjob(SJob::UpdateTreePaths);
                    self.feedsources_r
                        .borrow_mut()
                        .addjob(SJob::ScanEmptyUnread);
                    if new_id > 0 {
                        self.feedsources_r
                            .borrow_mut()
                            .addjob(SJob::ScheduleUpdateFeed(new_id));
                    }
                }
            }
            "import-opml" => {
                if let Some(AValue::ASTR(ref s)) = payload.get(0) {
                    (*self.feedsources_r)
                        .borrow_mut()
                        .import_opml(s.to_string());
                }
            }
            "export-opml" => {
                if let Some(AValue::ASTR(ref s)) = payload.get(0) {
                    let mut opmlreader = OpmlReader::new(self.subscriptionrepo_r.clone());
                    opmlreader.transfer_from_db();
                    match opmlreader.write_to_file(s.to_string()) {
                        Ok(()) => {
                            debug!("Writing {} success ", s);
                        }
                        Err(e) => {
                            warn!("Writing {} : {:?}", s, e);
                        }
                    }
                    debug!("export-opml :2   {:?}", &s);
                }
            }
            "feedsource-delete" => {
                debug!(
                    "feedsource-delete -> feedsource_move_to_trash()  {:?}",
                    &payload
                );
                self.feedsources_r.borrow_mut().feedsource_move_to_trash();
            }
            "subscription-edit-ok" => {
                debug!(" DialogData subscription-edit-ok  {:?}", &payload);
                self.feedsources_r
                    .borrow_mut()
                    .end_feedsource_edit_dialog(&payload);
            }
            "folder-edit" => {
                self.feedsources_r
                    .borrow_mut()
                    .end_feedsource_edit_dialog(&payload);
            }
            "settings" => {
                self.feedsources_r
                    .borrow_mut()
                    .set_conf_load_on_start(payload.get(0).unwrap().boo());
                self.feedsources_r
                    .borrow_mut()
                    .set_conf_fetch_interval(payload.get(1).unwrap().int().unwrap());
                self.feedsources_r
                    .borrow_mut()
                    .set_conf_fetch_interval_unit(payload.get(2).unwrap().int().unwrap());
                self.downloader_r
                    .borrow_mut()
                    .set_conf_num_threads(payload.get(3).unwrap().int().unwrap() as u8);
                self.feedcontents_r
                    .borrow_mut()
                    .set_conf_focus_policy(payload.get(4).unwrap().int().unwrap() as u8);
                self.feedsources_r
                    .borrow_mut() // 5 : DisplayCountOfAllFeeds
                    .set_conf_display_feedcount_all(payload.get(5).unwrap().boo());
                self.feedcontents_r
                    .borrow_mut()
                    .set_conf_msg_keep_count(payload.get(6).unwrap().int().unwrap());
                (*self.gui_context_r)
                    .borrow() // 7 : ManualFontSizeEnable
                    .set_conf_fontsize_manual_enable(payload.get(7).unwrap().boo());
                (*self.gui_context_r)
                    .borrow() // 8 : ManualFontSize
                    .set_conf_fontsize_manual(payload.get(8).unwrap().int().unwrap());
                (*self.browserpane_r)
                    .borrow_mut() // 9 : browser bg
                    .set_conf_browser_bg(payload.get(9).unwrap().uint().unwrap());
                (self.configmanager_r).borrow().set_val(
                    &PropDef::BrowserClearCache.to_string(),
                    payload.get(10).unwrap().boo().to_string(), // 10 : browser cache cleanup
                );
                (self.configmanager_r).borrow().set_val(
                    contentdownloader::CONF_DATABASES_CLEANUP, // 11 : DB cleanup
                    payload.get(11).unwrap().boo().to_string(),
                );

                let systray_e = payload.get(12).unwrap().boo();
                (self.configmanager_r).borrow().set_val(
                    &PropDef::SystrayEnable.to_string(),
                    systray_e.to_string(), // 12 : enable systray
                );
                self.addjob(Job::NotifyConfigChanged);
            }
            _ => {
                warn!("other DialogData: {:?}  {:?} ", &ident, payload);
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

    fn start_settings_dialog(&mut self) {
        let mut sources_conf = (*self.feedsources_r).borrow().get_config();
        if sources_conf.feeds_fetch_interval_unit == 0 {
            sources_conf.feeds_fetch_interval_unit = 3; // set to days if it was not set before
        }
        let downloader_conf = (*self.downloader_r).borrow().get_config();
        let contentlist_conf = (*self.feedcontents_r).borrow().get_config();
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

        //  (*self.gui_val_store)            .read()            .unwrap()            .get_gui_property_or(PropDef::SystrayEnable, "false".to_string())            .parse::<bool>()            .unwrap();

        let dd: Vec<AValue> = vec![
            AValue::ABOOL(sources_conf.feeds_fetch_at_start), // 0 : FetchFeedsOnStart
            AValue::AU32(sources_conf.feeds_fetch_interval),  // 1 UpdateFeeds Cardinal
            AValue::AU32(sources_conf.feeds_fetch_interval_unit), // 2 UpdateFeeds Unit:  1:minutes  2:hours  3:days
            AValue::AU32(downloader_conf.num_downloader_threads as u32), // 3 Web Fetcher Threads
            AValue::AI32(contentlist_conf.focus_policy as i32),   // 4 Message Focus Policy
            AValue::ABOOL(sources_conf.display_feedcount_all),    // 5 : DisplayCountOfAllFeeds
            AValue::AU32(contentlist_conf.message_keep_count as u32), // 6 Messages Keep Count
            AValue::ABOOL(fontsize_manual_enable),                // 7 : FontSizeManualEnable
            AValue::AU32(fontsize_manual),                        // 8 : Font size Manual
            AValue::AU32(browser_conf.browser_bg as u32),         // 9 : Browser_BG
            AValue::ABOOL(browser_cache_clear),                   // 10 : Browser Cache Cleanup
            AValue::ABOOL(databases_cleanup),                     // 11 : Cleanup-on-start
            AValue::ABOOL(systray_enable),                        // 12 : Systray enable
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

    #[allow(clippy::needless_range_loop)] // handle this later
    fn update_status_bar(&mut self) {
        const VERTICAL_RISING_BAR: [u32; 10] = [
            ' ' as u32, '_' as u32, 0x2581, 0x2582, 0x2583, 0x2584, 0x2585, 0x2586, 0x2587, 0x2588,
        ];
        let mut need_update1: bool = false;
        let mut need_update2: bool = false;
        let repo_id_new: isize;
        let mut last_fetch_time: i64 = 0;
        let mut feed_src_link = String::default();
        let mut is_folder: bool = false;
        let o_fse = (*self.feedsources_r)
            .borrow()
            .get_current_selected_subscription();
        if let Some((fse, _)) = o_fse {
            repo_id_new = fse.subs_id;
            last_fetch_time = fse.updated_int;
            feed_src_link = fse.url.clone();
            self.statusbar_items.num_downloader_threads = (*self.downloader_r)
                .borrow()
                .get_config()
                .num_downloader_threads;
            is_folder = fse.is_folder;
        } else {
            repo_id_new = -1;
        }
        let content_ids = (*self.feedcontents_r).borrow().get_selected_content_ids();
        let mut selected_msg_id = -1;
        if !content_ids.is_empty() {
            selected_msg_id = *content_ids.first().unwrap();
        }
        let mut num_msg_all = self.statusbar_items.num_msg_all;
        let mut num_msg_unread = self.statusbar_items.num_msg_unread;
        let subs_state: SubsMapEntry = (*self.feedsources_r)
            .borrow()
            .get_state(repo_id_new)
            .unwrap_or_default();
        if selected_msg_id != self.statusbar_items.selected_msg_id
            || repo_id_new != self.statusbar_items.selected_repo_id
        {
            self.statusbar_items.selected_msg_id = selected_msg_id;
        }
        if let Some((n_a, n_u)) = subs_state.num_msg_all_unread {
            num_msg_all = n_a;
            num_msg_unread = n_u;
            if n_a != self.statusbar_items.num_msg_all || n_u != self.statusbar_items.num_msg_unread
            {
                // trace!(                    "STATUS :   {}  unread/all ={}/{}",                   repo_id_new, num_msg_unread, num_msg_all                );
                need_update2 = true;
            }
        }

        if repo_id_new > 0 {
            if num_msg_all != self.statusbar_items.num_msg_all {
                self.statusbar_items.num_msg_all = num_msg_all;
                need_update1 = true;
            }
            if num_msg_unread != self.statusbar_items.num_msg_unread {
                self.statusbar_items.num_msg_unread = num_msg_unread;
                need_update1 = true;
            }

            if self.statusbar_items.last_fetch_time != last_fetch_time {
                self.statusbar_items.last_fetch_time = last_fetch_time;
                need_update2 = true;
            }
        }
        let last_msg_url = if selected_msg_id < 0 {
            String::default()
        } else {
            (self.browserpane_r).borrow().get_last_selected_link()
        };
        if self.statusbar_items.selected_msg_url != last_msg_url {
            self.statusbar_items.selected_msg_url = last_msg_url;
            need_update2 = true;
        }
        let longtext = if self.statusbar_items.selected_msg_url.is_empty() {
            string_escape_url(feed_src_link)
        } else {
            string_escape_url(self.statusbar_items.selected_msg_url.clone())
        };
        let mut block_vertical: char = ' ';
        if !is_folder && repo_id_new != self.statusbar_items.selected_repo_id {
            self.statusbar_items.selected_repo_id = repo_id_new;
            // time-to next feed update
            let fs_conf = self.feedsources_r.borrow().get_config();
            let interval_s = fs_conf.get_interval_seconds();
            let now = timestamp_now();
            let elapsed: i64 = std::cmp::min(now - (last_fetch_time as i64), interval_s);
            let div_idx = if interval_s == 0 {
                0
            } else {
                (elapsed * (8_i64)) / interval_s
            };
            block_vertical = char::from_u32(VERTICAL_RISING_BAR[div_idx as usize]).unwrap();
            need_update2 = true;
            need_update1 = true;
        }
        let downloader_busy = (self.downloader_r).borrow().get_kind_list();
        for a in 0..(DOWNLOADER_MAX_NUM_THREADS as usize) {
            if self.statusbar_items.downloader_kind[a] > 0 && downloader_busy[a] == 0 {
                self.statusbar_items.downloader_kind_new[a] = 0;
                need_update1 = true;
            }
            if self.statusbar_items.downloader_kind[a]
                != self.statusbar_items.downloader_kind_new[a]
            {
                self.statusbar_items.downloader_kind[a] =
                    self.statusbar_items.downloader_kind_new[a];
                need_update1 = true;
            }
        }

        let new_qsize = (*self.downloader_r).borrow().get_queue_size();
        if new_qsize != self.statusbar_items.num_dl_queue_length {
            self.statusbar_items.num_dl_queue_length = new_qsize;
            need_update1 = true;
        }
        if need_update1 {
            let mut downloader_display: String = String::default();
            for a in 0..(self.statusbar_items.num_downloader_threads as usize) {
                let nc = dl_char_for_kind(self.statusbar_items.downloader_kind[a] as u8);
                downloader_display.push(nc);
            }
            let unread_all = format!(
                "{:5} / {:5}",
                self.statusbar_items.num_msg_unread, self.statusbar_items.num_msg_all
            );
            let memdisplay = if self.statusbar_items.mode_debug {
                format!(
                    "  {}MB ",
                    self.statusbar_items.mem_usage_vmrss_bytes / 1048576,
                )
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
            let msg1tooltip = if self.statusbar_items.num_dl_queue_length > 0 {
                format!("Queue: {}", self.statusbar_items.num_dl_queue_length)
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
    }

    fn start_about_dialog(&mut self) {
        (*self.gui_updater).borrow().show_dialog(DIALOG_ABOUT);
    }

    ///  for key codes look at selec.rs                           gdk_sys::GDK_KEY_Escape => KeyCodes::Escape,
    fn process_key_press(&mut self, keycode: isize, _o_char: Option<char>) {
        let mut new_focus_by_tab = self.focus_by_tab.clone();
        let kc: KeyCodes = ui_select::from_gdk_sys(keycode);
        let subscription_id: isize = match (*self.feedsources_r)
            .borrow()
            .get_current_selected_subscription()
        {
            Some((subs_e, _)) => subs_e.subs_id,
            None => -1,
        };
        match kc {
            KeyCodes::Tab => new_focus_by_tab = self.focus_by_tab.next(),
            KeyCodes::ShiftTab => new_focus_by_tab = self.focus_by_tab.prev(),
            KeyCodes::Key_a => {
                if subscription_id > 0 {
                    // (*self.feedcontents_r).borrow_mut().set_read_all();
                    debug!("key:a {}", subscription_id);
                    (*self.feedsources_r).borrow().mark_as_read(subscription_id);
                }
            }
            KeyCodes::Key_s => {
                (*self.feedcontents_r)
                    .borrow_mut()
                    .move_list_cursor(ListMoveCommand::PreviousUnreadMessage);
            }
            KeyCodes::Key_x => {
                (*self.feedcontents_r)
                    .borrow_mut()
                    .move_list_cursor(ListMoveCommand::LaterUnreadMessage);
            }
            KeyCodes::Delete => {
                if self.focus_by_tab == FocusByTab::FocusMessages {
                    (*self.feedcontents_r).borrow().process_kb_delete();
                } else {
                    debug!("delete key but unfocused");
                }
            }
            _ => {
                // trace!("key-pressed: other {} {:?} {:?}", keycode, _o_char, kc);
            }
        }
        if new_focus_by_tab != self.focus_by_tab {
            self.focus_by_tab = new_focus_by_tab;
            self.switch_focus_marker(true);
            self.addjob(Job::CheckFocusMarker(2));
            // trace!("FOCUS:  {:?} ", &self.focus_by_tab );
            match &self.focus_by_tab {
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
        // trace!("switch_focus_marker: {:?} {:?} ", self.focus_by_tab, mark);
        match &self.focus_by_tab {
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

    // Mem usage in kb: current=105983, peak=118747411
    // Htop:  103M    SHR: 73580m   0,7% mem
    // top:	Res:106MB   SHR:78MB
    // estimation of the current physical memory used by the application, in bytes. 			Comes from proc//status/VmRSS
    pub fn update_memory_stats(&mut self) {
        if let Ok(mem) = proc_status::mem_usage() {
            self.statusbar_items.mem_usage_vmrss_bytes =
                (self.statusbar_items.mem_usage_vmrss_bytes + mem.current as isize) / 2;
        }
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
        _ => '_',
    };
    nc
}

impl Buildable for GuiProcessor {
    type Output = GuiProcessor;
    fn build(_conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
        let mut gp = GuiProcessor::new(_appcontext);
        gp.statusbar_items.mem_usage_vmrss_bytes = -1;
        gp
    }
}

impl TimerReceiver for GuiProcessor {
    fn trigger(&mut self, event: &TimerEvent) {
        if self.currently_minimized {
            match event {
                TimerEvent::Timer1s => {
                    self.process_event();
                }
                TimerEvent::Timer10s => {
                    self.update_memory_stats();
                    self.process_jobs();
                    self.update_status_bar();
                }
                _ => (),
            }
        } else {
            match event {
                TimerEvent::Timer100ms => {
                    self.process_event();
                    self.process_jobs();
                    self.update_status_bar();
                }
                TimerEvent::Timer1s => {
                    self.update_memory_stats();
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
        let gp_r = ac.get_rc::<GuiProcessor>().unwrap();
        {
            let mut t = (*self.timer_r).borrow_mut();
            t.register(&TimerEvent::Timer100ms, gp_r.clone());
            t.register(&TimerEvent::Timer1s, gp_r.clone());
            t.register(&TimerEvent::Timer10s, gp_r.clone());
            t.register(&TimerEvent::Startup, gp_r);
            self.timer_sender = Some((*t).get_ctrl_sender());
        }
        self.addjob(Job::StartApplication);
        self.addjob(Job::NotifyConfigChanged);
        self.statusbar_items.num_downloader_threads = (*self.downloader_r)
            .borrow()
            .get_config()
            .num_downloader_threads;

        if let Some(s) = (*self.configmanager_r)
            .borrow()
            .get_sys_val(ConfigManager::CONF_MODE_DEBUG)
        {
            if let Ok(b) = s.parse::<bool>() {
                self.statusbar_items.mode_debug = b;
            }
        }
    }
}

#[derive(Default)]
struct StatusBarItems {
    downloader_kind: [u8; DOWNLOADER_MAX_NUM_THREADS as usize],
    downloader_kind_new: [u8; DOWNLOADER_MAX_NUM_THREADS as usize],
    num_msg_all: isize,
    num_msg_unread: isize,
    last_fetch_time: i64,
    selected_repo_id: isize,
    num_downloader_threads: u8,
    num_dl_queue_length: usize,
    selected_msg_id: i32,
    selected_msg_url: String,
    /// proc//status/VmRSS  Resident set size, estimation of the current physical memory used by the application
    mem_usage_vmrss_bytes: isize,
    mode_debug: bool,
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
