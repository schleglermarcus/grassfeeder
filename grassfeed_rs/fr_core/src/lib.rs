#[allow(unused_imports)]
#[macro_use]
extern crate log;

#[cfg(test)]
extern crate mockall;
#[cfg(test)]
extern crate rand;

extern crate jpeg_decoder;
extern crate libwebp_image;


pub mod config;
pub mod controller;
pub mod db;
// pub mod ministatemachine;
pub mod opml;
pub mod timer;
pub mod ui_select;
pub mod util;
pub mod web;
pub mod downloader;
pub mod grassfeeder;


//  mit ui-gtk   :   22s	18s
//	ohne ui-gtk : 15s		10s
