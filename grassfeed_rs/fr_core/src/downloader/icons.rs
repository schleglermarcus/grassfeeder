use crate::controller::sourcetree::SJob;
use crate::db::icon_repo::IconEntry;
use crate::db::icon_repo::IconRepo;
use crate::downloader::util;
use crate::util::Step;
use crate::util::StepResult;
use crate::util::convert_webp_to_png;
use crate::web::WebFetcherType;
use flume::Sender;
use jpeg_decoder;


pub const ICON_CONVERT_TO_WIDTH: u32 = 32;

pub struct IconInner {
    pub fs_repo_id: isize,
    pub fs_icon_id_old: isize,
    pub feed_url: String,
    pub icon_url: String,
    pub icon_bytes: Vec<u8>,
    pub iconrepo: IconRepo,
    pub web_fetcher: WebFetcherType,
    pub download_error_happened: bool,
    pub sourcetree_job_sender: Sender<SJob>,
    pub feed_homepage: String,
    pub feed_download_text: String,
}

impl std::fmt::Debug for IconInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("")
            .field("repo_id", &self.fs_repo_id)
            //  .field("url", &self.icon_url)
            .field("feed", &self.feed_url)
            // .field("E", &self.download_error_happened)
            .finish()
    }
}

impl PartialEq for IconInner {
    fn eq(&self, other: &Self) -> bool {
        self.fs_repo_id == other.fs_repo_id
    }
}

pub struct IconLoadStart(IconInner);
impl IconLoadStart {
    pub fn new(i: IconInner) -> Self {
        IconLoadStart(i)
    }
}

impl Step<IconInner> for IconLoadStart {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        StepResult::Continue(Box::new(IconFeedTextDownload(self.0)))
    }
}

struct IconFeedTextDownload(IconInner);
impl Step<IconInner> for IconFeedTextDownload {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;
        let result = (*inner.web_fetcher).request_url(inner.feed_url.clone());
        // trace!(            "IconFeedTextDownload:1 {} {:?} icon_urL={}",            inner.feed_url,            result.status,            inner.icon_url        );
        match result.status {
            200 => {
                inner.feed_download_text = result.content;
            }
            _ => {
                //inner.download_error_happened = true;
                // trace!(                    "Feed download:  '{}' => {} {} {:?}  -> FallbackSimple ",                    &inner.feed_url,                    result.get_status(),                    result.get_kind(),                    result.error_description                );
                return StepResult::Continue(Box::new(IconFallbackSimple(inner)));
            }
        }
        if let (Some(homepage), Some(_feed_title)) = util::retrieve_homepage_from_feed_text(
            inner.feed_download_text.as_bytes(),
            &inner.feed_url,
        ) {
            inner.feed_homepage = homepage;
            // trace!(                "IconFeedTextDownload:2   HP={:?}  title={:?}",                inner.feed_homepage,                _feed_title            );
            return StepResult::Continue(Box::new(IconAnalyzeHomepage(inner)));
        }
        StepResult::Continue(Box::new(IconFallbackSimple(inner)))
    }
}

pub struct IconAnalyzeHomepage(IconInner);
impl Step<IconInner> for IconAnalyzeHomepage {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;
        // trace!(            "IconAnalyzeHomepage: {}   icon_url={}",           &inner.feed_homepage, inner.icon_url        );
        let r = (*inner.web_fetcher).request_url(inner.feed_homepage.clone());
        match r.status {
            200 => {
                if let Some(icon_url) =
                    util::extract_icon_from_homepage(r.content, &inner.feed_homepage)
                {
                    // trace!("extracted from page: {}", &icon_url);
                    inner.icon_url = icon_url;
                    return StepResult::Continue(Box::new(IconDownload(inner)));
                };
            }
            _ => {
                debug!(
                    "IconAnalyzeHomepage: {} {:?} {}",
                    inner.feed_homepage, r.status, r.error_description
                );
            }
        }
        StepResult::Continue(Box::new(IconFallbackSimple(inner)))
    }
}

struct IconFallbackSimple(IconInner);
impl Step<IconInner> for IconFallbackSimple {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner = self.0;
        if inner.icon_url.is_empty() {
            inner.icon_url = util::feed_url_to_icon_url(inner.feed_url.clone());
        }
        StepResult::Continue(Box::new(IconDownload(inner)))
    }
}

struct IconDownload(IconInner);
impl Step<IconInner> for IconDownload {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;
        let r = (*inner.web_fetcher).request_url_bin(inner.icon_url.clone());
        match r.status {
            200 => {
                // trace!(                    "IconDownload: OK {}  => #{} {} ",                    inner.fs_repo_id,                    &r.content_bin.len(),                    r.get_status(),                );
                inner.icon_bytes = r.content_bin;
                StepResult::Continue(Box::new(IconCheckIsImage(inner)))
            }
            _ => {
                inner.download_error_happened = true;
                debug!(
                    "IconDownload: {} {} '{}'  =>  {} {} {} -> STOP",
                    inner.fs_repo_id,
                    inner.feed_url,
                    inner.icon_url,
                    r.get_status(),
                    r.get_kind(),
                    r.error_description
                );
                StepResult::Stop(inner)
            }
        }
    }
}

pub struct IconCheckIsImage(pub IconInner);
impl Step<IconInner> for IconCheckIsImage {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let inner: IconInner = self.0;
        let (blob_is_icon, msg) = blob_is_icon(&inner.icon_bytes);
        if IsIconResult::IsWebp as usize == blob_is_icon {
            return StepResult::Continue(Box::new(IconWebpToPng(inner)));
        } else if blob_is_icon != 0 {
            debug!(
                "IconCheckIsImage: url={} length={} #feed_dl_text={}  Reason={}",
                inner.icon_url,
                inner.icon_bytes.len(),
                inner.feed_download_text.len(),
                msg
            );
            return StepResult::Stop(inner);
        }
        StepResult::Continue(Box::new(IconStore(inner)))
    }
}

pub struct IconWebpToPng(pub IconInner);
impl Step<IconInner> for IconWebpToPng {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;
        let r = convert_webp_to_png(&inner.icon_bytes, Some(ICON_CONVERT_TO_WIDTH));
        if r.is_none() {
            debug!(
                "icon image is webp, but failed to convert to png! {} ",
                inner.icon_url
            );
            return StepResult::Stop(inner);
        }
        inner.icon_bytes = r.unwrap();
        StepResult::Continue(Box::new(IconStore(inner)))
    }
}

struct IconStore(IconInner);
impl Step<IconInner> for IconStore {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let inner: IconInner = self.0;
        if inner.icon_bytes.len() < 10 {
            panic!("icon_too_small!");
        }
        //trace!(            "IconStore: {} {} len={}",            inner.icon_url,            inner.feed_url,            inner.icon_bytes.len()        );
        let comp_st = util::compress_vec_to_string(&inner.icon_bytes);
        let existing_icons: Vec<IconEntry> = inner.iconrepo.get_by_icon(comp_st.clone());
        if existing_icons.is_empty() {
            let ie = IconEntry {
                icon: comp_st,
                ..Default::default()
            };
            match inner.iconrepo.store_entry(&ie) {
                Ok(entry) => {
                    let _r = inner
                        .sourcetree_job_sender
                        .send(SJob::SetIconId(inner.fs_repo_id, entry.icon_id));
                }
                Err(e) => {
                    warn!("Storing Icon from {}  failed {:?}", inner.icon_url, e);
                }
            }
        } else {
            let existing_id = existing_icons[0].icon_id;
            if existing_id != inner.fs_icon_id_old {
                let _r = inner
                    .sourcetree_job_sender
                    .send(SJob::SetIconId(inner.fs_repo_id, existing_icons[0].icon_id));
            }
        }
        StepResult::Stop(inner)
    }
}

struct IconStop(IconInner);
impl Step<IconInner> for IconStop {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        StepResult::Stop(self.0)
    }
}

pub enum IsIconResult {
    IsIcon = 0,
    TooSmall = 1,
    NotIco = 2,
    NotPng = 4,
    NotBmp = 8,
    NotJpg = 16,
    NotSvg = 32,
    NotWebp = 64,
    IsWebp = 128,
}

///  Checks if this byte array is an icon
/// https://docs.rs/png_pong/latest/png_pong/struct.Decoder.html
pub fn blob_is_icon(vec_u8: &Vec<u8> /*, debug_msg: String*/) -> (usize, String) {
    let mut msg: String = String::default();
    let mut is_icon_result = 0;
    if vec_u8.len() < 10 {
        return (
            IsIconResult::TooSmall as usize,
            format!("blob: too small:{} ", vec_u8.len()),
        );
    }
    match ico::IconDir::read(std::io::Cursor::new(vec_u8)) {
        Ok(_decoder) => {
            return (0, msg);
        }
        Err(e) => {
            is_icon_result |= IsIconResult::NotIco as usize;
            msg = format!("{} not_ico: {}", msg, e);
        }
    }
    let cursor = std::io::Cursor::new(vec_u8.clone());
    match png_pong::Decoder::new(cursor) {
        Ok(_decoder) => {
            return (0, msg);
        }
        Err(e) => {
            is_icon_result |= IsIconResult::NotPng as usize;
            msg = format!("{} not_png: {}", msg, e);
        }
    }
    match tinybmp::RawBmp::from_slice(vec_u8) {
        Ok(_decoder) => {
            return (0, msg);
        }
        Err(e) => {
            is_icon_result |= IsIconResult::NotBmp as usize;

            msg = format!("{} not_bmp: {:?}", msg, e);
        }
    }
    let cursor = std::io::Cursor::new(vec_u8);
    let mut decoder = jpeg_decoder::Decoder::new(cursor);
    match decoder.decode() {
        Ok(_pixels) => {
            return (0, msg);
        }
        Err(e) => {
            is_icon_result |= IsIconResult::NotJpg as usize;
            msg = format!("{} not_jpg: {}", msg, e);
        }
    }
    match usvg::Tree::from_data(vec_u8, &usvg::Options::default().to_ref()) {
        Ok(_rtree) => {
            return (0, msg);
        }
        Err(e) => {
            is_icon_result |= IsIconResult::NotSvg as usize;
            msg = format!("{} not_svg: {}", msg, e);
        }
    }
    match libwebp_image::webp_load_from_memory(vec_u8) {
        Ok(_rtree) => {
            return (IsIconResult::IsWebp as usize, msg);
        }
        Err(e) => {
            is_icon_result |= IsIconResult::NotWebp as usize;
            msg = format!("{} not_webp: {}", msg, e);
        }
    }
    (is_icon_result, msg)
}

#[cfg(test)]
mod t_icons {
    use super::*;
    use crate::web::mockfilefetcher;

    //RUST_BACKTRACE=1 cargo watch -s "cargo test   controller::contentdownloader::downloader_test::test_is_icon  --lib -- --exact --nocapture "
    #[test]
    fn test_is_icon() {
        let set: [(&str, usize); 7] = [
            ("tests/data/feeds_seoulnews_favicon.ico", 126),
            ("tests/data/favicon.ico", 0),
            ("tests/data/icon_651.ico", 0),
            ("tests/data/report24-favicon.ico", 0),
            ("tests/data/naturalnews_favicon.ico", 0),
            ("tests/data/heise-safari-pinned-tab.svg", 0),
            ("tests/data/gorillavsbear_townsquare.ico", 0),
        ];
        set.iter().for_each(|(s, expected)| {
            let blob = mockfilefetcher::file_to_bin(s).unwrap();
            let (r, _msg) = blob_is_icon(&blob);
            println!("IS_ICON: {} {} {}", &s, r, _msg);
            assert_eq!(r, *expected);
        });
    }
}
