// mockall cannot provide a consistent data set, needs to be instrumented for each request separately.
mod downloader_dummy;
mod logger_config;
mod tree_drag_common;

// use crate::tree_drag_common::dataset_simple_trio;
// use crate::tree_drag_common::dataset_some_tree;
// use crate::tree_drag_common::dataset_three_folders;
// use crate::tree_drag_common::prepare_source_tree_controller;
// use fr_core::db::subscription_entry::SubscriptionEntry;

#[allow(dead_code)]
//#[test]
fn stub() {
    setup();
}

// ------------------------------------

#[allow(unused_imports)]
#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = logger_config::setup_logger();
    });
}
