use crate::minihttpserver::content_type::ContentType;
use chrono::offset::Local;
use chrono::prelude::DateTime;
use http::StatusCode;
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;

const SERVER_LOOP_TIME_MS: u64 = 100;
const HTTP_VERSION: &str = "HTTP/1.1";
const FILE_STREAM_BUFFER_SIZE: usize = 1000;

pub struct MiniHttpServerController {
    pub config: Arc<ServerConfig>,
    pub thread_handle: Option<JoinHandle<()>>,
    pub keeprunning: Option<Arc<AtomicBool>>,
    address: String,
}

impl MiniHttpServerController {
    pub fn new(conf: Arc<ServerConfig>) -> MiniHttpServerController {
        let addr = &(*conf).tcp_address.clone();
        MiniHttpServerController {
            config: conf,
            thread_handle: None,
            keeprunning: None,
            address: addr.clone(),
        }
    }

    pub fn start(&mut self) {
        match &self.thread_handle {
            Some(_s) => {
                debug!("mini already running !");
            }
            None => {
                let c = self.config.clone();
                let k = Arc::new(AtomicBool::new(true));
                self.keeprunning = Option::Some(k.clone());
                self.thread_handle = Option::Some(thread::spawn(move || {
                    let m = MiniHttpServer {
                        config: c,
                        keeprunning: k,
                    };
                    m.start();
                }));
            }
        }
    }

    pub fn stop(&mut self) {
        match &self.keeprunning {
            Some(k) => k.store(false, Ordering::Relaxed),
            None => {
                panic! {" thread spawned but no command-atomic, cannot stop !"}
            }
        }
        if self.thread_handle.is_some() {
            self.thread_handle.take().unwrap().join().unwrap();
        }
    }

    pub fn get_address(&self) -> String {
        format!("http://{}", &self.address)
    }
}

// #[derive(PartialEq)]  // geht net wegen Option<ContentType>
#[derive(Debug)]
pub enum AttachFileInfo {
    /// path, statuscode, length, contenttype
    FileInfoPath(String, StatusCode, u64, Option<ContentType>),
    LargeFile(String, StatusCode, u64, Option<ContentType>),
    ReplacementText(String, StatusCode),
    FileNotFound(Option<String>, StatusCode),
    InfoMessage(Option<String>, StatusCode),
    WarningMessage(Option<String>, StatusCode),
}

struct MiniHttpServer {
    pub config: Arc<ServerConfig>,
    pub keeprunning: Arc<AtomicBool>,
}

impl MiniHttpServer {
    pub fn start(&self) {
        let r = TcpListener::bind(&&self.config.tcp_address);
        if r.is_err() {
            error!(
                "Cannot bind address {} {:?}",
                &&self.config.tcp_address,
                r.err()
            );
            return;
        }
        let l = r.unwrap();
        let r = l.set_nonblocking(true);
        if r.is_err() {
            error!("Cannot set non-blocking {:?} ", r.err());
            return;
        }
        trace!("Starting Mini Server:    {:?} ", &self.config);
        self.loop_stream(l);
    }

    fn loop_stream(&self, listener: TcpListener) {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.handle_client(stream) {
                        error!("Error handling client: {}", e);
                    }
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    if !(self).keeprunning.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::yield_now();
                    std::thread::sleep(Duration::from_millis(SERVER_LOOP_TIME_MS));
                    continue;
                }
                Err(e) => error!("loop:  IO error: {}", e),
            }
        }
        trace!("Stopped Mini Server");
    }

    pub fn handle_client(&self, mut stream: TcpStream) -> std::io::Result<()> {
        let mut request_line = String::new(); // Get only the first line of the request, since this is a static HTTP 1.0 server.
        {
            let mut bf = BufReader::new(&stream);
            bf.read_line(&mut request_line)?;
        }
        let r = parse_request(&mut request_line);
        if r.is_err() {
            error!("Bad request: {}", &request_line);
            return Err(Error::from(ErrorKind::InvalidData));
        }
        let request = r.unwrap();
        let mut response: Response = self.build_response(&request);
        log_request(&request, &response.status);
        let resp_string = format_response(&response);
        let bytes_formatted = response_string_to_bytes(resp_string, response.body);
        stream.write_all(&bytes_formatted)?;
        if response.largefile.is_some() {
            stream.write_all(b"\n")?;
            let r = transfer_file(
                &mut stream,
                &response.largefile.unwrap(),
                self.config.download_throttling_kbps,
            );
            if let Ok(numwritten) = r {
                response.headers.content_length = numwritten;
            }
            // debug!(" transferdone: {:?}", response.headers);
        }

        Ok(())
    }

    fn build_response(&self, request: &Request) -> Response {
        let mut response = Response::new();
        if request.method != "GET" {
            response.status = StatusCode::METHOD_NOT_ALLOWED; // 405
        }
        let job = analyse_request(
            &self.config.htdocs_dir,
            self.config.binary_max_size,
            &self.config.index_file,
            &request.path,
        );
        let mut response_text_length: u64 = 0;
        match job {
            AttachFileInfo::InfoMessage(msg, sta) => {
                info!("{}", msg.unwrap());
                response.status = sta;
            }
            AttachFileInfo::WarningMessage(msg, sta) => {
                warn!("{}", msg.unwrap());
                response.status = sta;
            }
            AttachFileInfo::FileNotFound(msg, sta) => {
                debug!("{}", msg.unwrap());
                response.status = sta;
            }

            AttachFileInfo::ReplacementText(text, statusc) => {
                response.status = statusc;
                if response.status == StatusCode::OK {
                    response.body = Some(text.as_bytes().to_vec());
                    response_text_length = text.as_bytes().len() as u64;
                }
            }
            AttachFileInfo::FileInfoPath(path_str, statusc, file_length, contenttype) => {
                if let Some(ct) = contenttype {
                    response.headers.content_type = Some(ct);
                }
                response.status = statusc;
                add_file_to_response(&path_str, &mut response, file_length).unwrap();
            }
            AttachFileInfo::LargeFile(path_str, statusc, file_length, contenttype) => {
                if let Some(ct) = contenttype {
                    response.headers.content_type = Some(ct);
                }
                response.status = statusc;
                response.headers.content_length = file_length;
                response.largefile = Some(path_str);
            }
        }
        if response.status != StatusCode::OK && response.body.is_none() {
            let s: &str = response.status.canonical_reason().unwrap();
            let combi = format!("{} {}", response.status.as_str(), s);
            response.body = Some(combi.as_bytes().to_vec());
            response_text_length = combi.as_bytes().len() as u64;
        }
        if response.headers.content_length == 0 {
            response.headers.content_length = response_text_length;
        }
        response
    }
}

pub struct Response {
    body: Option<Vec<u8>>,
    headers: ResponseHeaders,
    status: StatusCode,
    largefile: Option<String>,
}

impl Response {
    pub fn new() -> Response {
        Response {
            body: None,
            headers: ResponseHeaders::new(),
            status: StatusCode::OK,
            largefile: None,
        }
    }

    pub fn get_body_string(&self) -> String {
        if self.body.is_some() {
            return String::from_utf8_lossy(self.body.as_ref().unwrap()).to_string();
        }
        String::default()
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("")
            .field(&self.get_body_string())
            .field(&self.headers)
            .field(&self.status)
            .finish()
    }
}

/// returns:   amount of written bytes
///
pub fn transfer_file<W: Write + Read>(
    stream: &mut W,
    src_path: &str,
    throttling_kbps: i64,
) -> Result<u64, Box<dyn std::error::Error>> {
    const DOWNLOAD_BLOCK_DELAY_MAX: u64 = 1000;
    let mut src_file = File::open(Path::new(&src_path))?;
    let mut buffer: [u8; FILE_STREAM_BUFFER_SIZE] = [0; FILE_STREAM_BUFFER_SIZE];
    let starttime: Instant = Instant::now();
    let mut writtensum: u64 = 0;
    let mut buffer_full: bool = true;
    let mut wait_ms = 1;
    while buffer_full {
        let numread = src_file.read(&mut buffer)?;
        let (buf_part, _) = buffer.split_at(numread);
        let numwritten: u64;
        let r = stream.write(buf_part);
        match r {
            Ok(nw) => {
                numwritten = nw as u64;
            }
            Err(e) => {
                debug!("downloading {:?} : {:?}", src_path, e);
                buffer_full = false;
                numwritten = 0;
            }
        }
        writtensum += numwritten as u64;
        if numread < FILE_STREAM_BUFFER_SIZE {
            buffer_full = false;
        } else if throttling_kbps > 0 {
            let duration_ms = starttime.elapsed().as_millis() as u64;
            let est_time_ms = writtensum / throttling_kbps as u64;
            if est_time_ms > duration_ms {
                wait_ms = est_time_ms - duration_ms;
                if wait_ms > DOWNLOAD_BLOCK_DELAY_MAX {
                    wait_ms = DOWNLOAD_BLOCK_DELAY_MAX
                }
            } else {
                wait_ms /= 2;
            }
            //            debug!(                "wsum:{}\t duration_ms={}\t  est_time_ms={}\t wait_ms={} ",                writtensum, duration_ms, est_time_ms, wait_ms,            );
            thread::sleep(Duration::from_millis(wait_ms));
        }
    }
    Ok(writtensum)
}

pub fn analyse_request(
    htdocs_dir: &str,
    binary_max_size: u64,
    index_file_name: &str,
    req_path: &str,
) -> AttachFileInfo {
    let mut path_expanded = req_path.to_string();
    // let mut path_expanded =     format!("{}", req_path);
    match check_request_dir(htdocs_dir, req_path) {
        (false, false, _) => {
            return AttachFileInfo::FileNotFound(
                Some(format!("Path does not exist: {}{}", &htdocs_dir, &req_path)),
                StatusCode::NOT_FOUND,
            );
        }
        (true, false, _) => {} // File
        (false, true, path_exp) => {
            path_expanded = path_exp;
            if check_create_dir_index(htdocs_dir, &path_expanded, index_file_name) {
                let det: String = create_direntry_text(htdocs_dir, &path_expanded);
                let folderlist_html = wrap_text_in_html(&det);
                return AttachFileInfo::ReplacementText(folderlist_html, StatusCode::OK);
            } else {
                path_expanded = format!("{}{}", &path_expanded, index_file_name);
            }
        }
        _ => {}
    }
    let file_info: AttachFileInfo =
        check_attachment_size(htdocs_dir, &path_expanded, binary_max_size);
    file_info
}

pub fn check_attachment_size(
    htdocs_dir: &str,
    request_path: &str,
    max_size: u64,
) -> AttachFileInfo {
    let path_str = format!("{}{}", &htdocs_dir, &request_path);

    let p = Path::new(&request_path);
    let f_md: fs::Metadata;
    match fs::metadata(&path_str) {
        Ok(md) => {
            f_md = md;
        }
        Err(e) => {
            return AttachFileInfo::WarningMessage(
                Some(format!("metadata path  error : {:?}  {:?}", &path_str, &e)),
                StatusCode::NOT_ACCEPTABLE,
            )
        }
    }
    let mut opt_cont_type: Option<ContentType> = None;
    if let Some(ex) = p.extension() {
        let ext: &str = ex.to_str().unwrap_or("");
        opt_cont_type = Some(ContentType::from_file_ext(ext));
    }
    let file_length: u64 = f_md.len();
    if file_length <= max_size {
        AttachFileInfo::FileInfoPath(path_str, StatusCode::OK, file_length, opt_cont_type)
    } else {
        AttachFileInfo::LargeFile(path_str, StatusCode::OK, file_length, opt_cont_type)
    }
}

/** precondition:  requestpath points to folder
 *  return:     true if no index file present and index needs to be created
 */
fn check_create_dir_index(htdocs_dir: &str, request_path: &str, index_file_name: &str) -> bool {
    let path_str = format!("{}{}{}", &htdocs_dir, &request_path, &index_file_name);
    let p_exp = Path::new(&path_str);
    !p_exp.exists()
}

// returns:
//   is_file
//   is_folder
//   requestpath completed with slash added
pub fn check_request_dir(htdocs_dir: &str, path: &str) -> (bool, bool, String) {
    let path_str = format!("{}{}", htdocs_dir, path);
    let p = Path::new(&path_str);
    if !p.exists() {
        return (false, false, path.to_string());
    }
    if p.is_file() {
        return (true, false, path.to_string());
    }
    if p.is_dir() {
        let retpath: String = if !path.ends_with('/') {
            format!("{}/", path)
        } else {
            path.to_string()
        };
        return (false, true, retpath);
    }
    (false, false, path.to_string())
}

#[derive(Default, Debug)]
pub struct ServerConfig {
    pub htdocs_dir: String,
    pub index_file: String,
    pub tcp_address: String,
    ///  Size in Bytes
    pub binary_max_size: u64,
    ///  -1 switches  throttling off
    pub download_throttling_kbps: i64,
}

#[derive(Debug)]
struct Request {
    http_version: String,
    method: String,
    path: String,
    time: DateTime<Local>,
}

#[derive(Debug)]
struct ResponseHeaders {
    content_type: Option<ContentType>,
    content_length: u64,
}
impl ResponseHeaders {
    fn new() -> ResponseHeaders {
        ResponseHeaders {
            content_type: None,
            content_length: 0,
        }
    }
}

fn parse_request(request: &mut str) -> Result<Request, ()> {
    let mut parts = request.split(' ');
    let method = match parts.next() {
        Some(method) => method.trim().to_string(),
        None => return Err(()),
    };
    let path = match parts.next() {
        Some(path) => path.trim().to_string(),
        None => return Err(()),
    };
    let http_version = match parts.next() {
        Some(version) => version.trim().to_string(),
        None => return Err(()),
    };
    let time = Local::now();
    Ok(Request {
        http_version,
        method,
        path,
        time,
    })
}

pub fn add_file_to_response(
    path_str: &str,
    response: &mut Response,
    file_size: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let path_r: &Path = Path::new(path_str);

    let mut f = fs::File::open(&path_r)?;
    let mut buffer = Vec::with_capacity(file_size as usize);
    match f.read_to_end(&mut buffer) {
        Ok(num_read) => {
            response.body = Some(buffer);
            response.headers.content_length = num_read as u64;
            // debug!("add_file_to_response : {:?}   {:?}  {}/{} bytes",&path,&response.get_body_string(),num_read,file_size);
        }
        Err(e) => {
            error!("add_file_to_response error : {:?}  {:?}", path_r, &e);
            response.status = match e.kind() {
                ErrorKind::NotFound => StatusCode::NOT_FOUND, // 404
                ErrorKind::PermissionDenied => StatusCode::FORBIDDEN, //  403
                ErrorKind::Other => StatusCode::INTERNAL_SERVER_ERROR, //  500
                _ => StatusCode::SERVICE_UNAVAILABLE,         // 503
            }
        }
    }
    Ok(())
}

fn format_response(response: &Response) -> String {
    let mut result: String;
    // let status_reason = match response.status.canonical_reason() {
    //     Some(reason) => reason,
    //     None => "",
    // };

    let status_reason = response.status.canonical_reason().unwrap_or("");

    result = format!(
        "{} {} {}\r\n",
        HTTP_VERSION,
        response.status.as_str(),
        status_reason,
    );
    result = format!("{}Allow: GET\r\n", result);
    result = format!(
        "{}content-length: {}\r\n",
        result, response.headers.content_length
    );
    if let Some(content_type) = &response.headers.content_type {
        result = format!("{}content-type: {}\r\n", result, content_type.value());
    }
    // return result.to_string();
    result
}

fn response_string_to_bytes(resp_string: String, mut body: Option<Vec<u8>>) -> Vec<u8> {
    let mut bytes = resp_string.as_bytes().to_vec();
    if body.is_none() {
        return bytes;
    }
    let bodybytes: Vec<u8> = body.take().unwrap();
    let mut bb_copy: Vec<u8> = vec![0; bodybytes.len()];
    //    Vec::with_capacity(bodybytes.len());
    //    bb_copy.resize(bodybytes.len(), 0);

    bb_copy.copy_from_slice(&bodybytes);
    bytes.append(&mut b"\n".to_vec());
    bytes.append(&mut bb_copy);
    bytes
}

fn create_direntry_text(htdocs_dir: &str, request_dir: &str) -> String {
    let combi = format!("{}{}", &htdocs_dir, &request_dir);
    let p: &Path = Path::new(&combi);
    let mut dirs: Vec<String> = Vec::new();
    let mut files: Vec<String> = Vec::new();
    if let Ok(readdir) = fs::read_dir(p) {
        for entry in readdir {
            if entry.is_err() {
                continue;
            }
            let pathbuf: PathBuf = entry.unwrap().path();
            let filename_os = pathbuf.file_name();
            if filename_os.is_none() {
                continue;
            }
            let filename = filename_os.unwrap().to_str();
            if filename.is_none() {
                continue;
            }
            let entryname = filename.unwrap().to_owned();
            if pathbuf.is_dir() {
                dirs.push(entryname);
            } else {
                files.push(entryname);
            }
        }
    }
    dirs.sort();
    files.sort();
    dirs.append(&mut files);
    let mut r = format!("Index of {} <hr/>\n", request_dir);
    for e in dirs {
        let line = format!("<a href=\"{}\">{}</a><br/>\n", e, e);
        r.push_str(&line);
    }
    r
}

fn wrap_text_in_html(payload: &str) -> String {
    let html_top :&str = "<!DOCTYPE html><html lang=\"de\">\n<head><meta charset=\"utf-8\"><title></title>  </head>\n<body>";
    let html_bottom: &str = "\n</body></html>";
    [html_top, payload, html_bottom].join("\n")
}

fn log_request(request: &Request, response_status: &StatusCode) {
    trace!(
        "[{}] \"{} {} {} : {}  \"",
        request.time,
        request.method,
        request.path,
        request.http_version,
        response_status.as_u16()
    );
}

//
