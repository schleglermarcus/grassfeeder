extern crate flume;
extern crate pango;
extern crate webkit2gtk;
#[macro_use]
extern crate log;
#[allow(unused_imports)]
#[macro_use]
extern crate rust_i18n;

pub mod cell_data_func;
pub mod dialogs;
pub mod statistics_list;
pub mod gtk_object_tree;
pub mod load_css;
pub mod messagelist;
pub mod treeview2;
pub mod util;

use rust_i18n::i18n;
i18n!("../resources/locales");
