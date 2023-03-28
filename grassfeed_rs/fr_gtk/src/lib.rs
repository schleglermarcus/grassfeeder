#[cfg(feature = "g3sources")]
extern crate app_g3sources as dd;

#[cfg(feature = "g3new")]
extern crate dd_g3new as dd;

extern crate rust_i18n;

use rust_i18n::i18n;


extern crate gio;

#[macro_use]
extern crate log;
extern crate itertools; // for key codes // background color

pub extern crate gdk_sys;


pub mod cell_data_func;
pub mod dialogs;
pub mod gtk_object_tree;
pub mod load_css;
pub mod messagelist;
pub mod treeview2;
pub mod util;


i18n!("../resources/locales");
