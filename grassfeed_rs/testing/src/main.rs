#[macro_use]
extern crate rust_i18n;

i18n!("../resources/locales");

fn main() {
    assert_eq!(t!("M_SETTINGS"), "Settings".to_string());
    if false {
        http_serve_short();
    }
}

extern crate chrono;
extern crate log;

use std::sync::Arc;
use std::time::Duration;
use testing::minihttpserver::minisrv;

fn http_serve_short() {
    let addr = String::from("127.0.0.1:8001");
    let conf = minisrv::ServerConfig {
        htdocs_dir: String::from("tests/htdocs"),
        index_file: String::from("index.html"),
        tcp_address: addr,
        binary_max_size: 100,
        download_throttling_kbps: 10,
    };
    let mut msc = minisrv::MiniHttpServerController::new(Arc::new(conf));
    msc.start();
    let wait_seconds = 200;
    println!("Server started, waiting {wait_seconds} seconds until shutdown. ");
    std::thread::sleep(Duration::from_millis(wait_seconds * 1000));
    msc.stop();
}
