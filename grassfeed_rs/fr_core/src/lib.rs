#[allow(unused_imports)]
#[macro_use]
extern crate log;

#[macro_use]
extern crate rust_i18n;

extern crate proc_status;
#[cfg(test)]
extern crate rand;

// extern crate m_libwebp_image as libwebp_image;

extern crate m_lz4_compression as lz4_compression;

// remapping those crates that need source replacement for debian
#[cfg(feature = "dd-g3old")]
extern crate dd_g3old as dd;

#[cfg(feature = "dd-g3old")]
use dd_g3old::m_feed_rs as feed_rs;

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

i18n!("../resources/locales");

pub const TD_BASE: &str = "../target/td/";
