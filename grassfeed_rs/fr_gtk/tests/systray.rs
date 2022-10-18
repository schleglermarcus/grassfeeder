mod logger_config;

use gtk::prelude::*;
use gtk::Application;
use gtk::ApplicationWindow;
use libappindicator::{AppIndicator, AppIndicatorStatus};

pub const ICON_PATH: &str = "/usr/share/pixmaps/grassfeeder/";
pub const ICON2: &str = "grassfeeder-indicator2"; // grassfeeder-indicator2.png

fn create_tray2() {
    // gtk::init().unwrap();
    let mut indicator = AppIndicator::new("libappindicator test application", "");
    indicator.set_status(AppIndicatorStatus::Active);
    indicator.set_icon_theme_path(ICON_PATH);
    indicator.set_icon(ICON2);
    let mut m = gtk::Menu::new();
    let mi = gtk::CheckMenuItem::with_label("Hello Rust!");
    mi.connect_activate(|_| {
        debug!("activate  ->  quit");
    });

    mi.connect_hide(|_m| debug!("MENU hide!"));
    mi.connect_focus(|_m, _n| {
        debug!("MENU focus!");
        gtk::Inhibit(false)
    });

    mi.connect_draw(|_m, _n| {
        debug!("MENU draw!");
        gtk::Inhibit(false)
    });

    m.append(&mi);
    indicator.set_menu(&mut m);
    m.show_all();
}

#[ignore]
#[test]
fn libapp1() {
    setup();

    let application = Application::builder()
        .application_id("test.systray")
        .build();

    application.connect_activate(move |app: &Application| {
        debug!("app connect_activate ");
        let win = ApplicationWindow::builder()
            .application(app)
            .default_width(320)
            .default_height(200)
            .title("Hello, World!")
            .build();
        create_tray2();
        win.show_all();
    });

    debug!("libapp1 app.run() ...");
    application.run();
    debug!("run() done ");
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
