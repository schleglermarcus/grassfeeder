// https://askubuntu.com/questions/254298/cant-get-iconify-and-deiconify-to-work-properly

mod logger_config;

use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use gtk::prelude::*;
use gtk::Application;
use gtk::ApplicationWindow;
use libappindicator::{AppIndicator, AppIndicatorStatus};
use std::sync::Arc;

pub const ICON_PATH: &str = "/usr/share/pixmaps/grassfeeder/";
pub const ICON2: &str = "grassfeeder-indicator2"; // grassfeeder-indicator2.png

#[ignore]
#[test]
fn libapp1() {
    setup();

    let application = Application::builder()
        .application_id("test.systray")
        .build();

    application.connect_activate(move |app: &Application| {
        debug!("app connect_activate ");
        let appwin = ApplicationWindow::builder()
            .application(app)
            .default_width(320)
            .default_height(200)
            .title("Hello, World!")
            .build();
        let is_minimized: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

        let is_mini_c = is_minimized.clone();
        appwin.connect_window_state_event(move |_win, ev_win_st: &gdk::EventWindowState| {
            let is_icon =
                (ev_win_st.new_window_state().bits() & gdk::WindowState::ICONIFIED.bits()) > 0;

            trace!("state: isicon: {}", is_icon);
            let is_mini = (*is_mini_c).load(Ordering::Relaxed);
            if is_icon != is_mini {
                (*is_mini_c).store(is_icon, Ordering::Relaxed);
            }
            gtk::Inhibit(false)
        });

        let mut indicator = AppIndicator::new("libappindicator test application", "");
        indicator.set_status(AppIndicatorStatus::Active);
        indicator.set_icon_theme_path(ICON_PATH);
        indicator.set_icon(ICON2);
        let mut m = gtk::Menu::new();
        let mi = gtk::MenuItem::with_label("Show/Hide");
        let win_c = appwin.clone();
        let act1 = gio::SimpleAction::new("sub_another", None);
        win_c.add_action(&act1);
        act1.connect_activate(|_act, _variant| {
            debug!("ACT1");
        });

        let is_mini_c = is_minimized.clone();
        mi.connect_activate(move |_mi| {
            let is_mini = (*is_mini_c).load(Ordering::Relaxed);
            if is_mini {
                debug!(" is_mini={}   call deiconify... ", is_mini);
                win_c.deiconify();
                win_c.present();
            } else {
                debug!(" is_mini={}   iconify ", is_mini);
                win_c.iconify();
            }
        });
        m.append(&mi);
        indicator.set_menu(&mut m);
        m.show_all();

        appwin.show_all();
    });
    application.run();
}

// ------------------------------------
#[allow(unused_imports)]
#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = logger_config::setup_logger();
    });
}
