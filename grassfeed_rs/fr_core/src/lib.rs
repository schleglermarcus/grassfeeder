
// #### Old
// remapping those crates that need source replacement for debian
// #[cfg(feature = "g3sources")]
// extern crate app_g3sources as dd;
// #[cfg(feature = "g3sources")]
// extern crate rust_i18n;
//  use dd::rust_i18n::i18n;

// #### New
// #[cfg(feature = "g3new")]
// #[allow(unused_imports)]
// remapping also for testing
// #[cfg(feature = "g3new")]
// extern crate dd_g3new as dd;



#[macro_use]
extern crate rust_i18n;
#[allow(unused_imports)]
#[macro_use]
extern crate log;
extern crate lz4_compression;
extern crate rusqlite;
extern crate feed_rs;
extern crate gif;
extern crate tl;
extern crate usvg;


// #[allow(unused_imports)]

use rust_i18n::i18n;

i18n!("../resources/locales");

// #[cfg(not(feature = "app-g3sources"))]
// extern crate dd_g3new as dd;

#[cfg(test)]
extern crate rand;

pub mod config;
pub mod controller;
pub mod db;
pub mod downloader;
pub mod opml;
pub mod ui_select;
pub mod util;
pub mod web;

pub const TD_BASE: &str = "../target/td/";
