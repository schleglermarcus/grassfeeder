// use std::path::Path;

use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};

//  Restore
//
//  Quit

#[test]
fn libapp1() {
    gtk::init().unwrap();
    let mut indicator = AppIndicator::new("libappindicator test application", "");
    indicator.set_status(AppIndicatorStatus::Active);

    // let p_cmd = Path::new(env!("CARGO_MANIFEST_DIR"));
    // let icon_path = p_cmd.join("examples");
    // let icon_path = p_cmd.join("src/icons/");
    // let icon_p_str = icon_path.to_str().unwrap();

    let icon_path = "/usr/share/pixmaps/grassfeeder/";

    println!("01    {:?}", icon_path);
    indicator.set_icon_theme_path(icon_path);

    // indicator.set_icon_full("rust-logo", "icon");
    //    indicator.set_icon_full("grassfeeder-indicator1", "icon");

    indicator.set_icon("grassfeeder-indicator2.png");
    // 							 04-grass-cut-2.png

    println!("02");

    let mut m = gtk::Menu::new();
    let mi = gtk::CheckMenuItem::with_label("Hello Rust!");
    mi.connect_activate(|_| {
        println!("activate  ->  quit");
        gtk::main_quit();
    });
    m.append(&mi);
    indicator.set_menu(&mut m);

    m.show_all();
    println!("main ....");
    gtk::main();

    println!("finished");
}
