extern crate chrono;
extern crate log;

use testing::minihttpserver::minisrv;

use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
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
    println!(
        "Server started, waiting {}  seconds until shutdown. ",
        wait_seconds
    );
    std::thread::sleep(Duration::from_millis(wait_seconds * 1000));

    msc.stop();
    Ok(())
}
