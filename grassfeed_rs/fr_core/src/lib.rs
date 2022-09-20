#[allow(unused_imports)]
#[macro_use]
extern crate log;

#[cfg(test)]
extern crate mockall;
#[cfg(test)]
extern crate rand;
#[macro_use]
extern crate rust_i18n;

extern crate jpeg_decoder;
extern crate libwebp_image;

pub mod config;
pub mod controller;
pub mod db;
pub mod downloader;
pub mod opml;
pub mod timer;
pub mod ui_select;
pub mod util;
pub mod web;

i18n!("../resources/locales");
