extern crate gio;
extern crate gtk;
#[macro_use]
extern crate log;
extern crate itertools;

pub extern crate gdk_sys; // for key codes

#[macro_use]
extern crate rust_i18n;

pub mod cell_data_func;
pub mod dialogs;
pub mod gtk_object_tree;
pub mod treeview2;
pub mod util;
pub mod load_css;

i18n!("../resources/locales");


