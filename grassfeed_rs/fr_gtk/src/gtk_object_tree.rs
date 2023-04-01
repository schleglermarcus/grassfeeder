// use dd::flume;
// use dd::webkit2gtk;
// #[cfg(feature = "g3sources")]
// #[cfg(feature = "g3sources")]
// use gtk::ToolButtonBuilder;
// #[cfg(not(feature = "g3sources"))]
// use dd::gtk;
// #[cfg(not(feature = "g3sources"))]
// use dd::gdk;
// #[cfg(not(feature = "g3sources"))]
// use gtk::builders::ToggleToolButtonBuilder;
// #[cfg(not(feature = "g3sources"))]
// use gtk::builders::ToolButtonBuilder;

#[cfg(feature = "legacy3gtk14")]
use gtk::NotebookBuilder;
#[cfg(feature = "legacy3gtk14")]
use gtk::ToolButtonBuilder;
#[cfg(feature = "legacy3gtk14")]
use gtk::ToggleToolButtonBuilder;

#[cfg(not(feature = "legacy3gtk14"))]
use gtk::builders::NotebookBuilder;
#[cfg(not(feature = "legacy3gtk14"))]
use gtk::builders::ToolButtonBuilder;
#[cfg(not(feature = "legacy3gtk14"))]
use gtk::builders::ToggleToolButtonBuilder;


use rust_i18n;
use crate::dialogs::create_dialogs;
use crate::load_css::TAB_MARKER_HEIGHT;
use crate::messagelist::create_listview;
use crate::util::process_string_to_image;
use crate::util::DragState;
use crate::util::EvSenderWrapper;
use crate::util::MOUSE_BUTTON_RIGHT;
use flume::Sender;
use gdk::EventButton;
use gtk::pango::WrapMode;
use gtk::prelude::GtkMenuItemExt;
use gtk::prelude::MenuShellExt;
use gtk::prelude::WidgetExt;
use gtk::prelude::*;
use gtk::Adjustment;
use gtk::Align;
use gtk::Button;
use gtk::ButtonBox;
use gtk::Container;
use gtk::Dialog;
use gtk::IconSize;
use gtk::Image;
use gtk::Label;
use gtk::Menu;
use gtk::MenuBar;
use gtk::MenuItem;
use gtk::Orientation;
use gtk::Paned;
use gtk::ResizeMode;
use gtk::ScrolledWindow;
use gtk::SearchEntry;
use gtk::ShadowType;
use gtk::ToggleToolButton;
use gtk::ToolButton;
use gtk::Toolbar;
use gui_layer::abstract_ui::BrowserEventType;
use gui_layer::abstract_ui::GuiEvents;
use gui_layer::gui_values::PropDef;
use resources::gen_icons;
use resources::id::*;
use rust_i18n::t;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::RwLock;
use ui_gtk::dialogdatadistributor::DialogDataDistributor;
use ui_gtk::gtkrunner::CreateBrowserConfig;
use ui_gtk::GtkGuiBuilder;
use ui_gtk::GtkObjectsType;
use webkit2gtk::traits::WebContextExt;
use webkit2gtk::traits::WebViewExt;
use webkit2gtk::WebContext;
use webkit2gtk::WebView;
use webkit2gtk::WebsiteDataManager;

const TOOLBAR_ICON_SIZE: i32 = 28;
const TOOLBAR_BORDER_WIDTH: u32 = 0;
const TOOLBAR_MARGIN: i32 = 0;
const RATIO_BROWSER_FONTSIZE_PERCENT: u32 = 140;

pub const ICON_PATH: &str = "/usr/share/pixmaps/grassfeeder/";
pub const ICON2: &str = "grassfeeder-indicator2";

thread_local!(
    pub static GLOB_CACHE: RefCell<GuiCacheValues> = RefCell::new(GuiCacheValues::default());
);

#[derive(Default)]
pub struct GuiCacheValues {
    pane0x: i32,
    pane1x: i32,
    pub col0w: i32,
    window_width: i32,
    window_height: i32,
    window_is_iconified: bool,
}

#[derive(Default)]
pub struct GtkObjectTree {
    pub initvalues: HashMap<PropDef, String>,
}

// https://gtk-rs.org/gtk-rs-core/stable/0.14/docs/pango/rectangle/struct.Rectangle.html
#[cfg(not(feature = "legacy3gtk14"))]
fn get_width_height(rectangle: &gtk::Rectangle) -> (i32, i32) {
    ((*rectangle).width(), (*rectangle).height())
}

// https://gtk-rs.org/gtk-rs-core/stable/0.15/docs/pango/struct.Rectangle.html
#[cfg(feature = "legacy3gtk14")]
fn get_width_height(rectangle: &gtk::Rectangle) -> (i32, i32) {
    ((*rectangle).width, (*rectangle).height)
}

///  this runs in the gtk thread
impl GtkGuiBuilder for GtkObjectTree {
    fn build_gtk(
        &self,
        gui_event_sender: Sender<GuiEvents>,
        gtk_obj_a: GtkObjectsType,
        ddd: &mut DialogDataDistributor,
    ) {
        const FRAME_RESIZE: bool = true; // should this child expand when the paned widget is resized.
        const FRAME_SHRINK: bool = true; // can this child be made smaller than its requisition.
        const NONE_ADJ: Option<&Adjustment> = None;
        let window: gtk::Window = (*gtk_obj_a).read().unwrap().get_window().unwrap();
        let esw = EvSenderWrapper(gui_event_sender.clone());
        window.connect_window_state_event(
            move |_w: &gtk::Window, ev_win_st: &gdk::EventWindowState| {
                let state_bits = ev_win_st.new_window_state().bits();
                let is_icon = (state_bits & gdk::WindowState::ICONIFIED.bits()) > 0;
                let last_iconified = GLOB_CACHE.with(|glob| glob.borrow().window_is_iconified);
                if is_icon != last_iconified {
                    // trace!("win-state-bits: {:#06x}   is-icon:{}", state_bits, is_icon);
                    GLOB_CACHE.with(|glob| {
                        glob.borrow_mut().window_is_iconified = is_icon;
                    });
                    esw.sendw(GuiEvents::WindowIconified(is_icon));
                }
                gtk::Inhibit(false)
            },
        );
        let esw = EvSenderWrapper(gui_event_sender.clone());
        crate::load_css::load_css();
        window.connect_size_allocate(move |_win, rectangle| {
            let (n_w, n_h) = get_width_height(&rectangle);
            // let n_w: i32 = (*rectangle).width();            let n_h: i32 = (*rectangle).height();
            let (last_w, last_h) =
                GLOB_CACHE.with(|glob| (glob.borrow().window_width, glob.borrow().window_height));
            if n_w != last_w || n_h != last_h {
                GLOB_CACHE.with(|glob| {
                    glob.borrow_mut().window_width = n_w;
                    glob.borrow_mut().window_height = n_h;
                });
                esw.sendw(GuiEvents::WindowSizeChanged(n_w, n_h));
            }
        });

        create_dialogs(gui_event_sender.clone(), gtk_obj_a.clone(), ddd);
        let drag_state = Rc::new(RwLock::new(DragState::default()));
        let box_top = gtk::Box::new(Orientation::Vertical, 0);
        box_top.set_widget_name("box_top");
        window.add(&box_top);
        let paned_top = Paned::new(Orientation::Horizontal);
        paned_top.set_wide_handle(true);
        paned_top.set_widget_name("paned_top");
        box_top.add(&paned_top);
        let box_1_v = gtk::Box::new(Orientation::Vertical, 0);
        box_1_v.set_widget_name("box_1_v");
        paned_top.pack1(&box_1_v, false, false);
        paned_top.set_size_request(20, -1);
        let esw = EvSenderWrapper(gui_event_sender.clone());
        paned_top.connect_leave_notify_event(move |paned_top: &Paned, _a2| {
            let newpos: i32 = paned_top.position();
            if newpos != GLOB_CACHE.with(|glob| glob.borrow().pane1x) {
                GLOB_CACHE.with(|glob| {
                    glob.borrow_mut().pane1x = newpos;
                });
                esw.sendw(GuiEvents::PanedMoved(1, newpos));
            }
            gtk::Inhibit(false)
        });
        let mode_debug = self.get_bool(PropDef::AppModeDebug);
        let p2p = self.get_int(PropDef::GuiPane2Pos, 120) as i32;
        paned_top.set_position(p2p);
        let box_2_h = gtk::Box::new(Orientation::Horizontal, 0);
        box_1_v.add(&box_2_h);

        let menubar = create_menubar(gui_event_sender.clone(), gtk_obj_a.clone(), mode_debug);
        box_2_h.pack_start(&menubar, false, false, 0);
        create_toolbar(gui_event_sender.clone(), gtk_obj_a.clone(), &box_2_h);
        box_2_h.set_spacing(-1);
        create_browser_toolbar(gui_event_sender.clone(), &box_2_h);

        let paned_1 = Paned::new(Orientation::Horizontal);
        paned_1.set_size_request(100, -1);
        paned_1.set_wide_handle(true);
        box_1_v.add(&paned_1);
        let scrolledwindow_0 = ScrolledWindow::new(NONE_ADJ, NONE_ADJ);
        scrolledwindow_0.set_widget_name("scrolledwindow_0");
        scrolledwindow_0.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic); // scrollbar-h, scrollbar-v
        scrolledwindow_0.set_vexpand(true);
        scrolledwindow_0.set_shadow_type(ShadowType::EtchedIn);
        scrolledwindow_0.set_min_content_width(50);
        let sourcetree = crate::treeview2::create_treeview(
            gui_event_sender.clone(),
            drag_state,
            gtk_obj_a.clone(),
        );
        scrolledwindow_0.add(&sourcetree);
        paned_1.pack1(&scrolledwindow_0, false, FRAME_SHRINK);
        paned_1.set_resize_mode(ResizeMode::Queue); // is it needed ?
        let esw = EvSenderWrapper(gui_event_sender.clone());
        paned_1.connect_position_notify(move |paned| {
            let newpos: i32 = paned.position();
            if newpos != GLOB_CACHE.with(|glob| glob.borrow().pane0x) {
                GLOB_CACHE.with(|glob| {
                    glob.borrow_mut().pane0x = newpos;
                });
                esw.sendw(GuiEvents::PanedMoved(0, newpos));
            }
        });
        let p1p = self.get_int(PropDef::GuiPane1Pos, 90) as i32;
        paned_1.set_position(p1p);
        let col1width = self.get_int(PropDef::GuiCol1Width, 200) as i32;
        let sort_col = self.get_int(PropDef::GuiList0SortColumn, 0);
        let sort_asc = self.get_bool(PropDef::GuiList0SortAscending);
        let content_treeview2 = create_listview(
            gui_event_sender.clone(),
            col1width,
            gtk_obj_a.clone(),
            sort_col as i32,
            sort_asc,
        );

        let scrolledwindow_1 = ScrolledWindow::new(NONE_ADJ, NONE_ADJ);
        scrolledwindow_1.set_widget_name("scrolledwindow_1");
        scrolledwindow_1.add(&content_treeview2);
        scrolledwindow_1.set_shadow_type(gtk::ShadowType::EtchedIn);
        scrolledwindow_1.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic); // scrollbar-h, scrollbar-v
        scrolledwindow_1.set_vexpand(true);
        paned_1.pack2(&scrolledwindow_1, FRAME_RESIZE, FRAME_SHRINK);
        let content_tab_widget = self.create_content_tabs_2(gtk_obj_a.clone());
        paned_top.pack2(&content_tab_widget, true, true); // resize  , shrink: yes

        // box_1_v
        let box3_status = gtk::Box::new(Orientation::Horizontal, 0);
        box3_status.set_widget_name("box3_status");
        // box_1_v
        box_top.add(&box3_status);
        let label_st1 = Label::new(Some("|____|"));
        label_st1.set_width_request(20);
        box3_status.add(&label_st1);
        let label_st2 = Label::new(Some(">some_url<"));
        label_st2.set_width_request(100);
        label_st2.set_selectable(true);
        label_st2.connect_button_press_event(|label2: &Label, evb: &EventButton| {
            if evb.button() == MOUSE_BUTTON_RIGHT {
                label2.select_region(0, -1); // select all
            }
            gtk::Inhibit(false)
        });
        let layout_st = gtk::Layout::new(NONE_ADJ, NONE_ADJ);
        layout_st.add(&label_st2);
        layout_st.set_width(100);
        layout_st.set_vexpand(false);
        layout_st.set_hexpand(true);
        box3_status.add(&layout_st);
        let label_st3 = Label::new(Some("___"));
        label_st3.set_width_request(10);
        box3_status.add(&label_st3);
        {
            let mut ret = (*gtk_obj_a).write().unwrap();
            ret.set_label(LABEL_STATUS_1, &label_st1);
            ret.set_label(LABEL_STATUS_2, &label_st2);
            ret.set_label(LABEL_STATUS_3, &label_st3);
            ret.set_paned(PANED_1_LEFT, &paned_1);
            ret.set_scrolledwindow(SCROLLEDWINDOW_0, &scrolledwindow_0);
            ret.set_scrolledwindow(SCROLLEDWINDOW_1, &scrolledwindow_1);
            //  ret.set_create_systray_fn(Box::new(create_systray_icon_3));
        }
        connect_keyboard(gui_event_sender, gtk_obj_a.clone());
    }
}

impl GtkObjectTree {
    fn get_int(&self, name: PropDef, defaul: usize) -> usize {
        if self.initvalues.is_empty() {
            error!("GtkObjectTree: gui_values not present.   {:?}", &name);
            return defaul;
        }
        match self.initvalues.get(&name) {
            Some(s) => match s.parse::<usize>() {
                Ok(i) => i,
                Err(_e) => {
                    warn!(
                        "GtkObjectTree: using default {} for {}",
                        defaul,
                        name.tostring()
                    );
                    defaul
                }
            },
            None => defaul,
        }
    }

    fn get_bool(&self, name: PropDef) -> bool {
        if self.initvalues.is_empty() {
            return false;
        }
        match self.initvalues.get(&name) {
            Some(b) => match b.parse::<bool>() {
                Ok(i) => i,
                Err(_e) => false,
            },
            None => false,
        }
    }

    // fn set_preferred_languages(&self, languages: &[&str])
    // fn set_spell_checking_enabled(&self, enabled: bool)
    fn create_content_tabs_2(&self, gtk_obj_a: GtkObjectsType) -> Container {
        let box1_v = gtk::Box::new(Orientation::Vertical, 0);

        let label_entry_link = Label::new(Some("-"));
        label_entry_link.set_halign(Align::Start);
        label_entry_link.set_wrap(true);
        box1_v.pack_start(&label_entry_link, false, false, 0);

        let box2_h = gtk::Box::new(Orientation::Horizontal, 0);
        box1_v.pack_start(&box2_h, false, false, 1);
        let label_author = Label::new(Some("-"));
        label_author.set_halign(Align::Start);
        label_author.set_wrap(true);
        label_author.set_line_wrap_mode(WrapMode::Word);
        box2_h.pack_start(&label_author, false, false, 5);

        let label_date = Label::new(Some("-"));
        label_date.set_halign(Align::Center);
        box2_h.pack_start(&label_date, false, false, 5);

        let label_subscription = Label::new(Some("-"));
        label_subscription.set_halign(Align::End);
        label_subscription.set_wrap(true);
        label_subscription.set_line_wrap_mode(WrapMode::Word);
        box2_h.pack_end(&label_subscription, false, false, 5);

        let label_cat = Label::new(Some("-"));
        label_cat.set_halign(Align::End);
        label_cat.set_wrap(true);
        label_cat.set_line_wrap_mode(WrapMode::Word);
        box1_v.pack_start(&label_cat, false, false, 0);

        let box3_h = gtk::Box::new(Orientation::Horizontal, 0);
        box3_h.set_height_request(TAB_MARKER_HEIGHT as i32);
        box3_h.set_widget_name("box_1");
        box1_v.pack_start(&box3_h, false, false, 1);

        let browserdir = self
            .initvalues
            .get(&PropDef::BrowserDir)
            .cloned()
            .unwrap_or_default();
        let fontsize_manual_enable_s = self
            .initvalues
            .get(&PropDef::GuiFontSizeManualEnable)
            .cloned()
            .unwrap_or_default();
        let mut o_fontsize_man: Option<u8> = None;
        if let Ok(fs_man_en) = fontsize_manual_enable_s.parse() {
            if fs_man_en {
                if let Some(fsm_s) = self.initvalues.get(&PropDef::GuiFontSizeManual) {
                    let fontsizemanual: u8 = fsm_s.parse().unwrap();
                    o_fontsize_man = Some(fontsizemanual);
                }
            }
        }
        let clear_cache: bool = self.get_bool(PropDef::BrowserClearCache);
        {
            let mut ret = (*gtk_obj_a).write().unwrap();
            ret.set_label(LABEL_BROWSER_MSG_DATE, &label_date);
            ret.set_label(LABEL_BROWSER_MSG_AUTHOR, &label_author);
            ret.set_label(LABEL_BROWSER_MSG_CATEGORIES, &label_cat);
            ret.set_label(LABEL_BROWSER_ENTRY_LINK, &label_entry_link);
            ret.set_label(LABEL_BROWSER_SUBSCRIPTION, &label_subscription);
            ret.set_box(BOX_CONTAINER_4_BROWSER, &box1_v);
            ret.set_box(BOX_CONTAINER_3_MARK, &box3_h);
            ret.set_create_webcontext_fn(
                Some(Box::new(create_webcontext)),
                &browserdir,
                BOX_CONTAINER_4_BROWSER,
                clear_cache,
                o_fontsize_man,
            );
            ret.set_create_webview_fn(Some(Box::new(create_webview)));
        }
        box1_v.upcast()
    }
}

pub fn create_webcontext(b_conf: CreateBrowserConfig) -> WebContext {
    let wconte: WebContext;
    if !b_conf.browser_dir.is_empty() {
        wconte = create_webcontext_dep(&b_conf.browser_dir);
        wconte.set_favicon_database_directory(Some(&b_conf.browser_dir));
    } else {
        error!("build_gtk BrowserDir missing!");
        wconte = WebContext::default().unwrap();
    }
    wconte.set_spell_checking_enabled(false);
    if b_conf.startup_clear_cache {
        wconte.clear_cache();
    }
    wconte
}

pub fn create_webview(
    w_context: &WebContext,
    manual_fontsize: Option<u8>,
    ev_se: Sender<GuiEvents>,
) -> WebView {
    let webview1: WebView = WebView::with_context(w_context);
    webview1.set_widget_name("webview_0");
    webview1.set_border_width(1);

    // TODO deactivated 0.14  //  webview1.set_background_color(&gtk::gdk::RGBA::new(0.5, 0.5, 0.5, 0.5));

    let mut wvs_b = webkit2gtk::SettingsBuilder::new()
        .enable_java(false)
        // .enable_media_capabilities(false)         // TODO deactivated 0.14
        //  .enable_javascript_markup(false)
        .enable_html5_local_storage(false)
        .enable_developer_extras(false)
        .enable_smooth_scrolling(true)
        .enable_webgl(false)
        .enable_xss_auditor(false);
    if let Some(fontsize) = manual_fontsize {
        let adapted_size: u32 = RATIO_BROWSER_FONTSIZE_PERCENT * fontsize as u32 / 100;
        wvs_b = wvs_b.default_font_size(adapted_size);
    }
    let webview_settings = wvs_b.build();
    webview1.set_settings(&webview_settings);
    // webview1.connect_web_process_crashed(|wv: &WebView| {        warn!("WebView Crashed! going back ...");        true     });
    let esw = EvSenderWrapper(ev_se);
    webview1.connect_estimated_load_progress_notify(move |wv: &WebView| {
        let progress = (wv.estimated_load_progress() * 256.0) as i32;
        esw.sendw(GuiEvents::BrowserEvent(
            BrowserEventType::LoadingProgress,
            progress,
        ));
    });
    webview1.connect_ready_to_show(|_wv: &WebView| {
        trace!("ready_to_show: {}", 0);
    });
    webview1
}

// gdk_sys::GDK_KEY_space			 gdk-sys / src / lib.rs
// https://gtk-rs.org/gtk3-rs/stable/latest/docs/gdk_sys/index.html
#[allow(clippy::if_same_then_else)]
fn connect_keyboard(g_ev_se: Sender<GuiEvents>, gtk_obj_a: GtkObjectsType) {
    let o_win = (*gtk_obj_a).read().unwrap().get_window();
    if o_win.is_none() {
        return;
    }
    let win = o_win.unwrap();
    let esw = EvSenderWrapper(g_ev_se);
    win.connect_key_press_event(move |_win, key| {
        let mut entry_has_focus: bool = false;
        if let Some(searchentry) = (*gtk_obj_a).read().unwrap().get_searchentry(SEARCH_ENTRY_0) {
            entry_has_focus = searchentry.has_focus();
        }
        if entry_has_focus {
            return Inhibit(false);
        }
        let keyval = key.keyval();
        let keystate = key.state();
        // trace!("            keypress: {:?} {:?} ", keyval, keystate);
        if keystate.intersects(gdk::ModifierType::CONTROL_MASK) {
            // debug!("! CONTROL_MASK Ctrl- ");
        } else if keystate.intersects(gdk::ModifierType::MOD1_MASK) {
            // debug!("! MOD1_MASK   Alt- ");
        } else if keystate.intersects(gdk::ModifierType::MOD4_MASK) {
            // debug!("! MOD4_MASK   Win-Right- ");
        } else if keystate.intersects(gdk::ModifierType::MOD5_MASK) {
            // debug!("! MOD5_MASK   AltGr- ");
        } else if keystate.intersects(gdk::ModifierType::SUPER_MASK) {
            // debug!("! SUPER_MASK   Win-Left- ");
        } else {
            esw.sendw(GuiEvents::KeyPressed(*keyval as isize, keyval.to_unicode()));
            if (*keyval) as i32 == gdk_sys::GDK_KEY_space {
                return Inhibit(true); // don't process space, but all other keys
            }
        }
        Inhibit(false)
    });
}

// MenuBar
//   MenuItem
//     Menu
//       MenuItem
pub fn create_menubar(
    g_ev_se: Sender<GuiEvents>,
    gtk_obj_a: GtkObjectsType,
    mode_debug: bool,
) -> MenuBar {
    let icons_dialog: Dialog = (*gtk_obj_a)
        .read()
        .unwrap()
        .get_dialog(DIALOG_ICONS)
        .unwrap()
        .clone();
    let menubar = MenuBar::new();
    menubar.set_border_width(TOOLBAR_BORDER_WIDTH);
    menubar.set_margin(TOOLBAR_MARGIN);
    {
        let m_file_item = MenuItem::with_label(&t!("M_FILE"));
        m_file_item.set_widget_name("M_FILE");
        menubar.append(&m_file_item);
        let menu_file = Menu::new();
        m_file_item.set_submenu(Some(&menu_file));
        let m_import_opml = MenuItem::with_label(&t!("M_IMPORT_OPML"));
        m_import_opml.set_widget_name("M_IMPORT_OPML");
        menu_file.add(&m_import_opml);
        let gtk_obj_a1 = gtk_obj_a.clone();
        m_import_opml.connect_activate(move |_m| {
            let opml_import_dialog: Dialog = (*gtk_obj_a1)
                .read()
                .unwrap()
                .get_dialog(DIALOG_OPML_IMPORT)
                .unwrap()
                .clone();
            opml_import_dialog.show();
        });
        let m_import_opml = MenuItem::with_label(&t!("M_EXPORT_OPML"));
        m_import_opml.set_widget_name("M_EXPORT_OPML");
        menu_file.add(&m_import_opml);
        let gtk_obj_a2 = gtk_obj_a.clone();
        m_import_opml.connect_activate(move |_m| {
            let opml_export_dialog: Dialog = (*gtk_obj_a2)
                .read()
                .unwrap()
                .get_dialog(DIALOG_OPML_EXPORT)
                .unwrap()
                .clone();
            opml_export_dialog.show();
        });

        let m_settings = MenuItem::with_label(&t!("M_SETTINGS"));
        m_settings.set_widget_name("M_SETTINGS");
        menu_file.add(&m_settings);
        let se = g_ev_se.clone();
        m_settings.connect_activate(move |_m| {
            se.send(GuiEvents::MenuActivate(_m.widget_name().to_string()))
                .unwrap();
        });

        let m_file_quit = MenuItem::with_label(&t!("M_FILE_QUIT"));
        m_file_quit.set_widget_name("M_FILE_QUIT");
        menu_file.add(&m_file_quit);
        let se = g_ev_se.clone();
        m_file_quit.connect_activate(move |_m| {
            se.send(GuiEvents::MenuActivate(_m.widget_name().to_string()))
                .unwrap();
        });
    }
    if false {
        let m_item = MenuItem::with_label(&t!("M_OPTIONS"));
        m_item.set_widget_name("M_OPTIONS");
        menubar.append(&m_item);
        let menu_sub = Menu::new();
        m_item.set_submenu(Some(&menu_sub));
        let m_settings = MenuItem::with_label(&t!("M_SETTINGS"));
        m_settings.set_widget_name("M_SETTINGS");
        menu_sub.add(&m_settings);
        let se = g_ev_se.clone();
        m_settings.connect_activate(move |_m| {
            se.send(GuiEvents::MenuActivate(_m.widget_name().to_string()))
                .unwrap();
        });
    }
    {
        let m_item = MenuItem::with_label(&t!("M_HELP"));
        m_item.set_widget_name("M_HELP");
        menubar.append(&m_item);
        let menu = Menu::new();
        m_item.set_submenu(Some(&menu));
        {
            let m_about = MenuItem::with_label(&t!("M_SHORT_HELP"));
            m_about.set_widget_name("M_SHORT_HELP");
            menu.add(&m_about);
            let esw = EvSenderWrapper(g_ev_se.clone());
            m_about.connect_activate(move |_m| {
                esw.sendw(GuiEvents::MenuActivate(_m.widget_name().to_string()));
            });
        }
        {
            let m_about = MenuItem::with_label(&t!("M_ABOUT"));
            m_about.set_widget_name("M_ABOUT");
            menu.add(&m_about);
            let esw = EvSenderWrapper(g_ev_se);
            m_about.connect_activate(move |_m| {
                esw.sendw(GuiEvents::MenuActivate(_m.widget_name().to_string()));
            });
        }

        if mode_debug {
            let m_icons = MenuItem::with_label(&t!("M_ICONS"));
            m_icons.set_widget_name("M_ICONS");
            menu.add(&m_icons);
            let icons_d = icons_dialog;
            m_icons.connect_activate(move |_m| {
                icons_d.show_all();
            });
        }
    }
    menubar
}

pub fn create_toolbar(
    g_ev_se: Sender<GuiEvents>,
    gtk_obj_a: GtkObjectsType,
    containing_box: &gtk::Box,
) {
    let toolbar = Toolbar::new();
    toolbar.set_height_request(16);
    toolbar.set_icon_size(IconSize::SmallToolbar);
    toolbar.set_margin(TOOLBAR_MARGIN);
    toolbar.set_border_width(TOOLBAR_BORDER_WIDTH);
    {
        let image = Image::new();
        process_string_to_image(
            gen_icons::ICON_20_FOLDER_NEW_48,
            &image,
            &String::default(),
            TOOLBAR_ICON_SIZE,
        );
        let button1: ToolButton = ToolButtonBuilder::new()
            .icon_widget(&image)
            .tooltip_text(&t!("TB_ADD_FOLDER"))
            .build();
        let new_folder_d: Dialog = (*gtk_obj_a)
            .read()
            .unwrap()
            .get_dialog(DIALOG_NEW_FOLDER)
            .unwrap()
            .clone();
        button1.connect_clicked(move |_b| {
            new_folder_d.show_all();
        });
        toolbar.insert(&button1, -1);
    }
    {
        let image = Image::new();
        process_string_to_image(
            gen_icons::ICON_10_RSS_ADD_32,
            &image,
            &String::default(),
            TOOLBAR_ICON_SIZE,
        );
        let but: ToolButton = ToolButtonBuilder::new()
            .icon_widget(&image)
            .tooltip_text(&t!("TB_ADD_FEED"))
            .build();
        toolbar.insert(&but, -1);
        let new_subscription_dialog: Dialog = (*gtk_obj_a)
            .read()
            .unwrap()
            .get_dialog(DIALOG_NEW_SUBSCRIPTION)
            .unwrap()
            .clone();
        but.connect_clicked(move |_b| {
            new_subscription_dialog.show_all();
        });
    }
    {
        let image = Image::new();
        process_string_to_image(
            gen_icons::ICON_18_RELOAD_32,
            &image,
            &String::default(),
            TOOLBAR_ICON_SIZE,
        );
        let but: ToolButton = ToolButtonBuilder::new()
            .icon_widget(&image)
            .tooltip_text(&t!("TB_REFRESH_ALL"))
            .build();
        toolbar.insert(&but, -1);
        let esw = EvSenderWrapper(g_ev_se.clone());
        but.connect_clicked(move |_b| {
            esw.sendw(GuiEvents::ToolBarButton("reload-feeds-all".to_string()));
        });
    }
    if false {
        let image = Image::new();
        process_string_to_image(
            gen_icons::ICON_34_DATA_XP2,
            &image,
            &String::default(),
            TOOLBAR_ICON_SIZE,
        );
        let but: ToolButton = ToolButtonBuilder::new()
            .icon_widget(&image)
            .tooltip_text("troubleshooting pane move")
            .build();
        toolbar.insert(&but, -1);
        let esw = EvSenderWrapper(g_ev_se.clone());
        but.connect_clicked(move |_b| {
            esw.sendw(GuiEvents::ToolBarButton(
                "toolbutton-troubleshoot1".to_string(),
            ));
        });
    }
    if false {
        let ttb1: ToggleToolButton = ToggleToolButtonBuilder::new().label("Special1").build();
        toolbar.insert(&ttb1, -1);
        let esw = EvSenderWrapper(g_ev_se.clone());
        ttb1.connect_active_notify(move |sw| {
            esw.sendw(GuiEvents::ToolBarToggle(
                "special1".to_string(),
                sw.is_active(),
            ));
        });
        let ttb2: ToggleToolButton = ToggleToolButtonBuilder::new().label("Special2").build();
        toolbar.insert(&ttb2, -1);
        let esw = EvSenderWrapper(g_ev_se.clone());
        ttb2.connect_active_notify(move |sw| {
            esw.sendw(GuiEvents::ToolBarToggle(
                "special2".to_string(),
                sw.is_active(),
            ));
        });
    }
    // toolbar
    containing_box.add(&toolbar);
    let searchentry: SearchEntry = SearchEntry::new();
    containing_box.add(&searchentry);
    searchentry.set_tooltip_text(Some(&t!("TB_FILTER_1")));
    searchentry.set_height_request(12);
    searchentry.set_vexpand(false);

    let esw = EvSenderWrapper(g_ev_se);
    searchentry.connect_changed(move |se: &SearchEntry| {
        esw.sendw(GuiEvents::SearchEntryTextChanged(
            SEARCH_ENTRY_0,
            se.buffer().text(),
        ));
    });
    {
        let mut ret = (*gtk_obj_a).write().unwrap();
        ret.set_searchentry(SEARCH_ENTRY_0, &searchentry);
    }
}

pub fn create_browser_toolbar(g_ev_se: Sender<GuiEvents>, containing_box: &gtk::Box) {
    let toolbar = Toolbar::new();
    toolbar.set_height_request(16);
    toolbar.set_icon_size(IconSize::SmallToolbar);
    toolbar.set_margin(TOOLBAR_MARGIN);
    toolbar.set_border_width(TOOLBAR_BORDER_WIDTH);
    {
        let image = Image::new();
        process_string_to_image(
            gen_icons::ICON_36_ZOOM_IN,
            &image,
            &String::default(),
            TOOLBAR_ICON_SIZE,
        );
        let but: ToolButton = ToolButtonBuilder::new()
            .icon_widget(&image)
            .tooltip_text(&t!("TB_BROWSER_ZOOM_IN"))
            .build();
        toolbar.insert(&but, -1);
        let esw = EvSenderWrapper(g_ev_se.clone());
        but.connect_clicked(move |_b| {
            esw.sendw(GuiEvents::ToolBarButton("browser-zoom-in".to_string()));
        });
    }
    {
        let image = Image::new();
        process_string_to_image(
            gen_icons::ICON_40_ZOOM_FIT_BEST,
            &image,
            &String::default(),
            TOOLBAR_ICON_SIZE,
        );
        let but: ToolButton = ToolButtonBuilder::new()
            .icon_widget(&image)
            .tooltip_text(&t!("TB_BROWSER_ZOOM_DEFAULT"))
            .build();
        toolbar.insert(&but, -1);
        let esw = EvSenderWrapper(g_ev_se.clone());
        but.connect_clicked(move |_b| {
            esw.sendw(GuiEvents::ToolBarButton("browser-zoom-default".to_string()));
        });
    }
    {
        let image = Image::new();
        process_string_to_image(
            gen_icons::ICON_38_ZOOM_OUT,
            &image,
            &String::default(),
            TOOLBAR_ICON_SIZE,
        );
        let but: ToolButton = ToolButtonBuilder::new()
            .icon_widget(&image)
            .tooltip_text(&t!("TB_BROWSER_ZOOM_OUT"))
            .build();
        toolbar.insert(&but, -1);
        let esw = EvSenderWrapper(g_ev_se);
        but.connect_clicked(move |_b| {
            esw.sendw(GuiEvents::ToolBarButton("browser-zoom-out".to_string()));
        });
    }
    containing_box.pack_end(&toolbar, false, false, 0);
}

pub fn create_buttonbox(_g_ev_se: Sender<GuiEvents>) -> ButtonBox {
    let buttonbox = ButtonBox::new(Orientation::Horizontal);
    let button1: Button = Button::with_label("button1");
    buttonbox.add(&button1);
    buttonbox
}

#[cfg(not(feature = "legacy3gtk14"))]
pub fn create_webcontext_dep(browser_dir: &str) -> WebContext {
    let wk_dm = WebsiteDataManager::builder()
        .base_cache_directory(browser_dir)
        .base_data_directory(browser_dir)
        .disk_cache_directory(browser_dir)
        .hsts_cache_directory(browser_dir)
        .indexeddb_directory(browser_dir)
        .local_storage_directory(browser_dir)
        .build();
    WebContext::with_website_data_manager(&wk_dm)
}

#[cfg(feature = "legacy3gtk14")]
pub fn create_webcontext_dep(browser_dir: &str) -> WebContext {
    let wk_dm = WebsiteDataManager::builder().build();
    WebContext::builder().build()
}

// ---
