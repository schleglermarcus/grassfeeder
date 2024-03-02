extern crate bincode;
extern crate bmp;
extern crate feed_rs;
extern crate gif;
extern crate image;
extern crate jpeg_decoder;
extern crate libwebp_image;
extern crate lz4_compression;
extern crate png;
extern crate proc_status;
extern crate rusqlite;
extern crate tl;
extern crate webbrowser;
#[macro_use]
extern crate rust_i18n;
#[allow(unused_imports)]
#[macro_use]
extern crate log;
extern crate fontdb;
extern crate resvg;
extern crate tiny_skia;
extern crate usvg;

use rust_i18n::i18n;
i18n!("../resources/locales");

pub mod config;
pub mod controller;
pub mod db;
pub mod downloader;
pub mod opml;
pub mod ui_select;
pub mod util;
pub mod web;

/// test data base folder
pub const TD_BASE: &str = "../target/td/";
