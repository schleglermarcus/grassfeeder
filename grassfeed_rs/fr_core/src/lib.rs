#[allow(unused_imports)]
#[macro_use]
extern crate log;


/*
// remapping those crates that need source replacement for debian
#[cfg(feature = "dd-g3new")]
extern crate dd_g3new as dd;
 */



// remapping also for testing
extern crate dd_g3new as dd;


#[cfg(test)]
extern crate rand;

// Old
// use dd::rust_i18n::i18n;


// New
#[macro_use]
extern crate rust_i18n;
i18n!("../resources/locales");  





// #[cfg(feature = "ui-gtk")]
//  extern crate proc_status;
//  use dd_g3new::proc_status;
// extern crate m_libwebp_image as libwebp_image;
//  extern crate m_lz4_compression as lz4_compression;

// #[cfg(feature = "dd-g3old")]
// use dd_g3old::m_feed_rs as feed_rs;

// #[cfg(not(feature = "app-g3sources"))]
// extern crate dd_g3new as dd;

pub mod config;
pub mod controller;
pub mod db;
pub mod downloader;
pub mod opml;
pub mod ui_select;
pub mod util;
pub mod web;


pub const TD_BASE: &str = "../target/td/";
