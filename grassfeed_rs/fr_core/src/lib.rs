#[allow(unused_imports)]
#[macro_use]
extern crate log;
#[macro_use]
extern crate rust_i18n;
extern crate jpeg_decoder;
extern crate libwebp_image;
// extern crate nix;
extern crate proc_status;

// #[cfg(test)]
// extern crate mockall;
#[cfg(test)]
extern crate rand;


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
