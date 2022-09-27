use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Instant;
use testing::minihttpserver::content_type;
use testing::minihttpserver::minisrv;

// #[ignore]
#[test]
fn transfer_file_throttled() {
    setup();
    // let src_file_name = "tests/ms_htdocs/big/Win10_1511_1_German_x64.iso".to_string();
    let src_file_name: String = "tests/ms_htdocs/minisrv.rs".to_string();
    let throttling_kbps: i64 = 50;
    let mut rwmock = ReadWriteMock {};
    let start = Instant::now();
    let r = minisrv::transfer_file(&mut rwmock, &src_file_name, throttling_kbps);
    let duration_ms: u64 = start.elapsed().as_millis() as u64;
    if r.is_err() {
        error!("FAIL  {:?}   E={:?}", src_file_name, &r.err());
        assert!(false);
        return;
    }
    assert!(duration_ms > 0);
    let rough_size: u64 = 19000;
    let run_tp: u64 = rough_size / duration_ms;
    // debug!("run_tp={:}  ", run_tp);
    assert!(run_tp < (throttling_kbps as u64 + 1));
}

struct ReadWriteMock;
impl std::io::Write for ReadWriteMock {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Read for ReadWriteMock {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

#[test]
fn transfer_file_simple() {
    setup();
    let src_file_name = "tests/ms_htdocs/minisrv.rs".to_string();
    let mut rwmock = ReadWriteMock {};
    let tr = minisrv::transfer_file(&mut rwmock, &src_file_name, -1);
    match tr {
        Ok(num) => {
            assert!(num > 19000);
        }
        Err(e) => {
            error!("e={:}", e);
            assert!(false);
        }
    }
}

// #[ignore]
#[test]
fn analyse_request_withfile() {
    setup();
    let job = minisrv::analyse_request("tests/ms_htdocs/", 1000, "index.html", "index.html");
    match job {
        minisrv::AttachFileInfo::FileInfoPath(msg, sc, length, _o_contenttype) => {
            assert_eq!(msg, "tests/ms_htdocs/index.html".to_string());
            assert_eq!(length, 169);
            assert_eq!(sc, 200);
            assert!(_o_contenttype.is_some());
        }
        _ => {
            assert!(false, "wrong job {:?}", &job);
        }
    }
}

// #[ignore]
#[test]
fn analyse_request_nofile() {
    setup();
    let job = minisrv::analyse_request("tests/ms_htdocs/", 1000, "index.html", "none_");
    match job {
        minisrv::AttachFileInfo::FileNotFound(msg, _sta) => {
            assert!(msg.is_some());
        }
        _ => {
            assert!(false, "wrong job {:?}", &job);
        }
    }
}

// #[ignore]
#[test]
fn check_attachment_size_test() {
    setup();
    let t = minisrv::check_attachment_size("tests/ms_htdocs/", "file2.txt", 10000);
    match t {
        minisrv::AttachFileInfo::FileInfoPath(stri, sc, size, oct) => {
            assert_eq!(stri, "tests/ms_htdocs/file2.txt");
            assert_eq!(sc, 200);
            assert_eq!(size, 7);
            assert_eq!(content_type::ContentType::TEXT, oct.unwrap());
        }
        _ => {
            assert!(false);
        }
    }
}

// #[ignore]
#[test]
fn check_request_dir_test() {
    setup();
    let (file, folder, _ret) = minisrv::check_request_dir("tests/ms_htdocs/", "");
    assert!(!file);
    assert!(folder);
    let (file, folder, ret) = minisrv::check_request_dir("tests/ms_htdocs", "");
    assert!(!file);
    assert!(folder);
    assert_eq!("/", ret);
    let (file, folder, _ret) = minisrv::check_request_dir("tests/ms_htdocs/", "index.html");
    assert!(file);
    assert!(!folder);
}

#[test]
fn add_file_to_response_favicon() {
    setup();
    let mut response = minisrv::Response::new();
    minisrv::add_file_to_response("tests/ms_htdocs/favicon.ico", &mut response, 100).unwrap();
    assert!(response.get_body_string().len() > 10);
}

#[test]
fn add_file_to_response_text() {
    setup();
    let mut response = minisrv::Response::new();
    minisrv::add_file_to_response("tests/ms_htdocs/file2.txt", &mut response, 100).unwrap();
    assert_eq!("file_2\n", response.get_body_string());
}

#[test]
fn read_file_simple() {
    setup();
    let bufsize = 100;
    let path = Path::new("tests/ms_htdocs/file2.txt");
    let mut f = File::open(&path).unwrap();
    let mut buffer = Vec::with_capacity(bufsize as usize);
    let _n = f.read_to_end(&mut buffer).unwrap();
    let s = String::from_utf8_lossy(&buffer);
    assert_eq!("file_2\n", s);
}

// ------------------------------------

#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = testing::logger_config_local::setup_logger();
    });
}
