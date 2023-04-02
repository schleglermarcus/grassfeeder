// use dd::flume;

use crate::gtkmodel_updater::GtkModelUpdaterInt;
use crate::gtkrunner::GtkObjectsImpl;
use crate::DialogDataDistributor;
use crate::GtkBuilderType;
use crate::GtkObjectsType;
use crate::IntCommands;
use flume::Receiver;
use flume::Sender;
use gtk::gio::ApplicationFlags;
use gtk::gio::Cancellable;
use gtk::prelude::ApplicationExt;
use gtk::prelude::ApplicationExtManual;
use gtk::prelude::Cast;
use gtk::prelude::CellRendererSpinnerExt;
use gtk::prelude::GtkWindowExt;
use gtk::prelude::TreeViewColumnExt;
use gtk::prelude::WidgetExt;
use gtk::traits::SettingsExt;
use gtk::ApplicationWindow;
use gtk::Settings;
use gui_layer::abstract_ui::GuiEvents;
use gui_layer::abstract_ui::UIAdapterValueStoreType;
use gui_layer::abstract_ui::UiSenderWrapperType;
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use std::time::Instant;

const GTK_MAIN_INTERVAL: std::time::Duration = Duration::from_millis(100);
static INTERVAL_COUNTER: AtomicU8 = AtomicU8::new(0);

// https://gtk-rs.org/gtk-rs-core/stable/0.14/docs/glib/struct.MainContext.html
// https://github.com/gtk-rs/gtk-rs-core/issues/186
// https://github.com/gtk-rs/glib/issues/448
#[cfg(feature = "legacy3gtk14")]
macro_rules! g14m_guard{
    ()=>{
        let mc_ =gtk::glib::MainContext::default();
        let guard = mc_.acquire();
        if guard.is_err() {
            panic!("Special workaround for gtk0.14:   cannot aquire main context! exiting the timer loop ");
        }
    }
}

#[cfg(not(feature = "legacy3gtk14"))]
macro_rules! g14m_guard {
    () => {};
}

pub struct GtkRunnerInternal {
    gui_event_sender: Sender<GuiEvents>,
    pub gtk_objects: GtkObjectsType,
}

impl GtkRunnerInternal {
    pub fn new(ev_se: Sender<GuiEvents>) -> GtkRunnerInternal {
        GtkRunnerInternal {
            gui_event_sender: ev_se,
            gtk_objects: Arc::new(RwLock::new(GtkObjectsImpl::default())),
        }
    }

    /// Creates the window, connects the   activate signal with the build command
    /// https://gtk-rs.org/gtk-rs-core/stable/0.15/docs/gio/struct.ApplicationFlags.html#associatedconstant.HANDLES_COMMAND_LINE
    /// return  true on success.  False on   App-Was-Running
    pub fn init(
        &mut self,
        builder: &GtkBuilderType,
        win_title: String,
        win_width: i32,
        win_height: i32,
        app_url: String,
    ) -> bool {
        let ev_se = self.gui_event_sender.clone();
        let obj_c = self.gtk_objects.clone();
        let obj_c2 = self.gtk_objects.clone();

        let builder_c = builder.clone();
        let mut appflags: ApplicationFlags = ApplicationFlags::default();
        appflags.set(ApplicationFlags::HANDLES_COMMAND_LINE, false);
        let app = gtk::Application::new(Some(&app_url), appflags);
        dbus_register(&app);
        if app.is_remote() {
            warn!(
                "{} is already running (registered at d-bus, remote=1). Stopping. ",
                &app_url
            );
            let _r = ev_se.send(GuiEvents::AppWasAlreadyRunning);
            app.release();
            dbus_close(&app);
            return false;
        }
        (*obj_c).write().unwrap().set_application(&app);
        app.connect_activate(move |app| {
            let appwindow = build_window(app, &ev_se, win_title.clone(), win_width, win_height);
            let window: &gtk::Window = appwindow.upcast_ref::<gtk::Window>();
            let mut dd = DialogDataDistributor::default();
            (*obj_c).write().unwrap().set_window(window);
            (*obj_c)
                .write()
                .unwrap()
                .set_gui_event_sender(ev_se.clone());
            (*builder_c).build_gtk(ev_se.clone(), obj_c2.clone(), &mut dd);
            (*obj_c).write().unwrap().set_dddist(dd);
            window.show_all();
        });
        true
    }

    /// this  blocks the caller completely, while running the app
    /// https://gtk-rs.org/gtk-rs-core/stable/0.15/docs/gio/prelude/trait.ApplicationExtManual.html#tymethod.run
    ///  LATER: find a way how to process both sets of parameters:   application   AND gtk
    pub fn run(&self) {
        let app_o = (*self.gtk_objects).read().unwrap().get_application();
        match app_o {
            Some(appli) => {
                let run_result = appli.run_with_args::<String>(&[]);
                if run_result != 0 {
                    error!("PROBLEM on gtk:application.run() {}", run_result);
                }
            }
            None => {
                panic!("run(): no gtk application !");
            }
        }
    }

    pub fn add_timeout_loop(
        g_com_rec: Receiver<IntCommands>,
        gtk_objects: GtkObjectsType,
        model_value_store: UIAdapterValueStoreType,
        ev_se_w: UiSenderWrapperType,
    ) {
        let gtk_objects_a = gtk_objects.clone();
        // let gtk_objects_b = gtk_objects.clone();
        let m_v_st_a = model_value_store.clone();
        let upd_int = GtkModelUpdaterInt::new(model_value_store, gtk_objects, ev_se_w);
        let is_minimized: AtomicBool = AtomicBool::new(false);
        g14m_guard!();
        gtk::glib::timeout_add_local(GTK_MAIN_INTERVAL, move || {
            let prev_count = INTERVAL_COUNTER.fetch_add(1, Ordering::Relaxed);
            if is_minimized.load(Ordering::Relaxed) && (prev_count & 7 != 0) {
                return gtk::glib::Continue(true);
            }
            let mut rec_set: HashSet<IntCommands> = HashSet::new();
            while let Ok(command) = g_com_rec.try_recv() {
                rec_set.insert(command);
            }
            let mut rec_list = rec_set.iter().collect::<Vec<_>>();
            rec_list.sort();
            for command in rec_list {
                let now = Instant::now();
                // trace!("  INT: #{}   ", prev_count);
                match *command {
                    IntCommands::START => {
                        error!("glib loop: unexpected START ");
                    }
                    IntCommands::STOP => {
                        match (gtk_objects_a).read().unwrap().get_window() {
                            Some(win) => {
                                win.close();
                            }
                            None => {
                                error!("glib loop: STOP cannot close application window ");
                            }
                        };
                    }
                    IntCommands::UpdateTextEntry(i) => upd_int.update_text_entry(i),
                    IntCommands::UpdateTreeModel(i) => upd_int.update_tree_model(i),
                    IntCommands::UpdateTreeModelSingle(tree_nr, ref path) => {
                        upd_int.update_tree_model_single(tree_nr, path.clone())
                    }
                    IntCommands::TreeSetCursor(i, ref path) => {
                        upd_int.tree_set_cursor(i, path.clone())
                    }
                    IntCommands::UpdateListModel(i) => upd_int.update_list_model(i),
                    IntCommands::UpdateListModelSingle(i, list_pos) => {
                        upd_int.update_list_model_single(i, list_pos)
                    }
                    IntCommands::UpdateListModelSome(i, ref list_pos) => {
                        upd_int.update_list_model_some(i, list_pos.clone())
                    }
                    IntCommands::ListSetCursor(i, pos, column, scroll_pos) => {
                        upd_int.list_set_cursor(i, pos, column, scroll_pos)
                    }
                    IntCommands::UpdateTextView(i) => upd_int.update_text_view(i),
                    IntCommands::UpdateLabel(i) => upd_int.update_label(i),
                    IntCommands::UpdateLabelMarkup(i) => upd_int.update_label_markup(i),
                    IntCommands::UpdateDialog(i) => upd_int.update_dialog(i),
                    IntCommands::UpdateLinkButton(i) => upd_int.update_linkbutton(i),
                    IntCommands::ShowDialog(i) => upd_int.show_dialog(i),
                    IntCommands::UpdatePanedPos(i, pos) => upd_int.update_paned_pos(i, pos),
                    IntCommands::WidgetMark(ref typ, i, mark) => {
                        upd_int.widget_mark(typ.clone(), i, mark);
                    }
                    IntCommands::GrabFocus(ref typ, i) => {
                        upd_int.grab_focus(typ.clone(), i);
                    }
                    IntCommands::UpdateWindowTitle => upd_int.update_window_title(),
                    IntCommands::UpdateWindowIcon => upd_int.update_window_icon(),
                    IntCommands::UpdateWebView(_i) => {
                        if !upd_int.update_web_view() {
                            (gtk_objects_a).write().unwrap().set_web_view(None, None);
                        }
                    } // only one view
                    IntCommands::UpdateWebViewPlain(_i) => upd_int.update_web_view_plain(),
                    IntCommands::ClipBoardSetText(ref s) => {
                        gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD).set_text(s);
                    }
                    IntCommands::WebViewRemove(fs_man) => {
                        (gtk_objects_a).write().unwrap().set_web_view(None, fs_man);
                    }
                    IntCommands::MemoryConserve(act) => {
                        is_minimized.store(act, Ordering::Relaxed);
                        upd_int.memory_conserve(act);
                    }
                    IntCommands::TrayIconEnable(_act) => {
                        //  upd_int.update_tray_icon(act);
                    }
                    IntCommands::UpdateWindowMinimized(mini, ev_time) => {
                        upd_int.update_window_minimized(mini, ev_time)
                    }

                    _ => {
                        warn!("GTKS other cmd {:?}", command);
                    }
                }
                let elapsed_ms = now.elapsed().as_millis();
                if elapsed_ms > 200 {
                    warn!("R_INT: {:?} took {:?}", &command, elapsed_ms);
                }
            }
            if prev_count & 1 == 0 {
                if let Some((cr_spinner, tv_col)) = (*gtk_objects_a).read().unwrap().get_spinner_w()
                {
                    let m_sp_act = (*m_v_st_a).read().unwrap().is_spinner_active();
                    cr_spinner.set_active(m_sp_act);
                    if m_sp_act {
                        let mut pulse = cr_spinner.pulse();
                        pulse = (pulse + 1) % 12;
                        cr_spinner.set_pulse(pulse);
                        let alignment = match pulse & 1 {
                            0 => 0.0,
                            _ => 0.5,
                        };
                        tv_col.set_alignment(alignment);
                    }
                }
            }
            gtk::glib::Continue(true)
        });
    } // timeout
}

fn build_window(
    app: &gtk::Application,
    event_sender: &Sender<GuiEvents>,
    title: String,
    width: i32,
    height: i32,
) -> ApplicationWindow {
    let window = gtk::ApplicationWindow::new(app);
    window.set_title(&title);
    window.set_default_size(width, height);
    window.show_all();
    if let Some(screen) = window.screen() {
        if let Some(settings) = Settings::for_screen(&screen) {
            settings.set_gtk_enable_animations(true);
        } else {
            debug!("INT: build_window()  got no settings from gtk screen! ")
        }
    }
    let g_ev_se2 = event_sender.clone();
    window.connect_delete_event(move |_a, _b| {
        g_ev_se2.send(GuiEvents::WinDelete).unwrap();
        gtk::Inhibit(false)
    });
    window
}

#[cfg(not(feature = "legacy3gtk14"))]
pub fn dbus_close(app: &gtk::Application) {
    if let Some(dbuscon) = app.dbus_connection() {
        dbuscon.close(gtk::gio::Cancellable::NONE, |_a1| {
            debug!("GtkRunnerInternal: dbus-closed callback");
        });
    }
}

#[cfg(feature = "legacy3gtk14")]
pub fn dbus_close(app: &gtk::Application) {
    if let Some(dbuscon) = app.dbus_connection() {
        let none_cancellable: Option<&Cancellable> = Option::None;
        dbuscon.close(none_cancellable, |_a1| {
            debug!("GtkRunnerInternal: dbus-closed callback");
        });
    }
}

#[cfg(not(feature = "legacy3gtk14"))]
pub fn dbus_register(app: &gtk::Application) {
    let _r = app.register(Cancellable::NONE);
}

#[cfg(feature = "legacy3gtk14")]
pub fn dbus_register(app: &gtk::Application) {
    let none_cancellable: Option<&Cancellable> = Option::None;
    let _r = app.register(none_cancellable);
}
