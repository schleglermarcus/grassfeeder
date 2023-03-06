extern crate gio;
extern crate gtk;
#[macro_use]
extern crate log;
extern crate itertools; // for key codes // background color
// extern crate libappindicator;

pub extern crate gdk_sys;

#[macro_use]
extern crate rust_i18n;

pub mod cell_data_func;
pub mod dialogs;
pub mod gtk_object_tree;
pub mod load_css;
pub mod messagelist;
// pub mod systray_icon;
pub mod treeview2;
pub mod util;

i18n!("../resources/locales");
