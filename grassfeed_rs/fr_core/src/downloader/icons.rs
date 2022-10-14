use crate::controller::sourcetree::SJob;
use crate::db::errors_repo::ErrorRepo;
use crate::db::icon_repo::IconEntry;
use crate::db::icon_repo::IconRepo;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::downloader::util;
use crate::downloader::util::workaround_https_declaration;
use crate::util::convert_webp_to_png;
use crate::util::downscale_png;
use crate::util::Step;
use crate::util::StepResult;
use crate::web::WebFetcherType;
use flume::Sender;
use jpeg_decoder;

pub const ICON_CONVERT_TO_WIDTH: u32 = 48;

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
    pub subscriptionrepo: SubscriptionRepo,
    pub erro_repo: ErrorRepo,
}

impl std::fmt::Debug for IconInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("")
            .field("repo_id", &self.fs_repo_id)
            .field("feed", &self.feed_url)
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
        let mut inner: IconInner = self.0;
        if let Some(subs_e) = inner.subscriptionrepo.get_by_index(inner.fs_repo_id) {
            if !subs_e.website_url.is_empty() {
                inner.feed_homepage = subs_e.website_url;
                return StepResult::Continue(Box::new(IconAnalyzeHomepage(inner)));
            }
        }
        StepResult::Continue(Box::new(FeedTextDownload(inner)))
    }
}

struct FeedTextDownload(IconInner);
impl Step<IconInner> for FeedTextDownload {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;
        let result = (*inner.web_fetcher).request_url(inner.feed_url.clone());
        match result.status {
            200 => {
                inner.feed_download_text = result.content;
            }
            _ => {
                inner.erro_repo.add_error(
                    inner.fs_repo_id,
                    result.status as isize,
                    inner.feed_url.clone(),
                    result.error_description,
                );
                return StepResult::Continue(Box::new(IconFallbackSimple(inner)));
            }
        }
        StepResult::Continue(Box::new(HomepageDownload(inner)))
    }
}

struct HomepageDownload(IconInner);
impl Step<IconInner> for HomepageDownload {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;
        let dl_text = workaround_https_declaration(inner.feed_download_text.clone());
        if let (Some(homepage), Some(_feed_title)) =
            util::retrieve_homepage_from_feed_text(dl_text.as_bytes(), &inner.feed_url)
        {
            inner.feed_homepage = homepage;
            return StepResult::Continue(Box::new(CompareHomepageToDB(inner)));
        }
        StepResult::Continue(Box::new(IconFallbackSimple(inner)))
    }
}

struct CompareHomepageToDB(IconInner);
impl Step<IconInner> for CompareHomepageToDB {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let inner: IconInner = self.0;

        if let Some(subs_e) = inner.subscriptionrepo.get_by_index(inner.fs_repo_id) {
            if !inner.feed_homepage.is_empty() && inner.feed_homepage != subs_e.website_url {
                inner
                    .subscriptionrepo
                    .update_homepage(inner.fs_repo_id, &inner.feed_homepage);
            }
        } else {
            debug!("no subscription in db for {}", inner.fs_repo_id);
        }
        StepResult::Continue(Box::new(IconAnalyzeHomepage(inner)))
    }
}

pub struct IconAnalyzeHomepage(IconInner);
impl Step<IconInner> for IconAnalyzeHomepage {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;
        let r = (*inner.web_fetcher).request_url(inner.feed_homepage.clone());
        match r.status {
            200 => {
                if let Some(icon_url) =
                    util::extract_icon_from_homepage(r.content, &inner.feed_homepage)
                {
                    inner.icon_url = icon_url;
                    return StepResult::Continue(Box::new(IconDownload(inner)));
                };
            }
            _ => {
                inner.erro_repo.add_error(
                    inner.fs_repo_id,
                    r.status as isize,
                    inner.feed_homepage.clone(),
                    r.error_description,
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
                inner.icon_bytes = r.content_bin;
                StepResult::Continue(Box::new(IconCheckIsImage(inner)))
            }
            _ => {
                inner.download_error_happened = true;
                // trace!(                    "IconDownload: {} {} '{}'  =>  {} {} {} -> STOP",                    inner.fs_repo_id,                    inner.feed_url,                    inner.icon_url,                    r.get_status(),                    r.get_kind(),                    r.error_description                );
                inner.erro_repo.add_error(
                    inner.fs_repo_id,
                    r.get_status() as isize,
                    inner.icon_url.clone(),
                    format!("kind:{}   {}", r.get_kind(), r.error_description),
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
        let an_res = icon_analyser(&inner.icon_bytes);
        if an_res.kind == IconKind::Webp {
            return StepResult::Continue(Box::new(IconWebpToPng(inner)));
        }
        if an_res.width_orig > ICON_CONVERT_TO_WIDTH || an_res.height_orig > ICON_CONVERT_TO_WIDTH {
            if an_res.kind == IconKind::Png {
                return StepResult::Continue(Box::new(IconPngDownscale(inner)));
            } else {
                warn!(
                    "IconCheckIsImage:2:  {}x{} {:?} {} ",
                    an_res.width_orig, an_res.height_orig, an_res.kind, inner.icon_url,
                );
            }
        }

        if an_res.kind == IconKind::UnknownType || an_res.kind == IconKind::TooSmall {
            // trace!(                "IconCheckIsImage: url={} length={} #feed_dl_text={}  Reason={}",                inner.icon_url,                inner.icon_bytes.len(),               inner.feed_download_text.len(),                an_res.message            );
            inner.erro_repo.add_error(
                inner.fs_repo_id,
                inner.icon_bytes.len() as isize,
                inner.icon_url.clone(),
                an_res.message,
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
        if r.is_err() {
            inner.erro_repo.add_error(
                inner.fs_repo_id,
                0,
                inner.icon_url.clone(),
                format!("convert-webp-to-png {} {:?}", inner.icon_url, r.err()),
            );
            return StepResult::Stop(inner);
        }
        inner.icon_bytes = r.unwrap();
        StepResult::Continue(Box::new(IconStore(inner)))
    }
}

pub struct IconPngDownscale(pub IconInner);
impl Step<IconInner> for IconPngDownscale {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;
        let r = downscale_png(&inner.icon_bytes, ICON_CONVERT_TO_WIDTH);
        if r.is_err() {
            let msg = format!("png-downscale:{} {:?}", inner.icon_url, r.err());
            warn!("{msg}");
            inner
                .erro_repo
                .add_error(inner.fs_repo_id, 0, inner.icon_url.clone(), msg);
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
        if inner.icon_bytes.len() > 4000 {
            debug!(
                "IconStore: {} {} \tlen={}",
                inner.icon_url,
                inner.feed_url,
                inner.icon_bytes.len()
            );
        }
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
                    error!("Storing Icon from {}  failed {:?}", inner.icon_url, e);
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
#[deprecated]
pub fn blob_is_icon(vec_u8: &Vec<u8>) -> (usize, String) {
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

    /*
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
    */
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

    match tinybmp::RawBmp::from_slice(vec_u8) {
        Ok(_decoder) => {
            return (0, msg);
        }
        Err(e) => {
            is_icon_result |= IsIconResult::NotBmp as usize;

            msg = format!("{} not_bmp: {:?}", msg, e);
        }
    }

    (is_icon_result, msg)
}

#[derive(Debug, PartialEq)]
pub enum IconKind {
    None,
    TooSmall,
    Ico,
    Png,
    Bmp,
    Jpg,
    Svg,
    Webp,
    UnknownType, // all analyses done
}

impl Default for IconKind {
    fn default() -> Self {
        IconKind::None
    }
}

#[derive(Debug, Default)]
pub struct IconAnalyseResult {
    pub kind: IconKind,
    width_orig: u32,
    height_orig: u32,
    pub message: String,
    // _rescaled_png: Vec<u8>,
}

impl IconAnalyseResult {
    pub fn new(k: IconKind) -> IconAnalyseResult {
        IconAnalyseResult {
            kind: k,

            ..Default::default()
        }
    }
    pub fn with_msg(k: IconKind, msg: String) -> IconAnalyseResult {
        IconAnalyseResult {
            kind: k,
            message: msg,
            ..Default::default()
        }
    }
}

pub fn icon_analyser(vec_u8: &Vec<u8>) -> IconAnalyseResult {
    let analysers: [Box<dyn InvestigateOne>; 7] = [
        Box::new(BySize {}),
        Box::new(InvJpg {}),
        Box::new(InvIco {}),
        Box::new(InvPng {}),
        Box::new(InvSvg {}),
        Box::new(InvWebp {}),
        Box::new(InvBmp {}),
    ];
    let mut msgs: Vec<String> = Vec::default();
    for a in analysers {
        let r: IconAnalyseResult = a.investigate(vec_u8);
        if r.kind != IconKind::None {
            return r;
        }
        msgs.push(r.message);
    }
    IconAnalyseResult::with_msg(IconKind::UnknownType, msgs.join(" "))
}

trait InvestigateOne {
    fn investigate(&self, blob: &Vec<u8>) -> IconAnalyseResult;
}

struct BySize {}
impl InvestigateOne for BySize {
    fn investigate(&self, vec_u8: &Vec<u8>) -> IconAnalyseResult {
        let mut r = IconAnalyseResult::default();
        if vec_u8.len() < 10 {
            r.kind = IconKind::TooSmall;
            r.message = format!("too small, length:{} ", vec_u8.len());
        }
        r
    }
}

struct InvIco {}
impl InvestigateOne for InvIco {
    fn investigate(&self, vec_u8: &Vec<u8>) -> IconAnalyseResult {
        let mut r = IconAnalyseResult::default();
        match ico::IconDir::read(std::io::Cursor::new(vec_u8)) {
            Ok(decoder) => {
                r.kind = IconKind::Ico;
                if let Some(entry) = decoder.entries().first() {
                    r.width_orig = entry.width();
                    r.height_orig = entry.height();
                }
            }
            Err(e) => {
                r.message = format!("not_ico: {}", e);
            }
        }
        r
    }
}

struct InvPng {}
impl InvestigateOne for InvPng {
    fn investigate(&self, vec_u8: &Vec<u8>) -> IconAnalyseResult {
        let mut r = IconAnalyseResult::default();
        let cursor = std::io::Cursor::new(vec_u8.clone());
        let decoder = png::Decoder::new(cursor);
        match decoder.read_info() {
            Ok(mut reader) => {
                r.kind = IconKind::Png; // Allocate the output buffer.
                let mut buf = vec![0; reader.output_buffer_size()]; // Read the next frame. An APNG might contain multiple frames.
                if let Ok(info) = reader.next_frame(&mut buf) {
                    r.width_orig = info.width;
                    r.height_orig = info.height;
                }
            }
            Err(e) => {
                r.message = format!("not_png: {}", e);
            }
        }
        r
    }
}

struct InvJpg {}
impl InvestigateOne for InvJpg {
    fn investigate(&self, vec_u8: &Vec<u8>) -> IconAnalyseResult {
        let mut r = IconAnalyseResult::default();
        let cursor = std::io::Cursor::new(vec_u8);
        let mut decoder = jpeg_decoder::Decoder::new(cursor);
        match decoder.decode() {
            Ok(_pixels) => {
                r.kind = IconKind::Ico;
            }
            Err(e) => {
                r.message = format!("not_jpg: {}", e);
            }
        }
        r
    }
}

struct InvSvg {}
impl InvestigateOne for InvSvg {
    fn investigate(&self, vec_u8: &Vec<u8>) -> IconAnalyseResult {
        let mut r = IconAnalyseResult::default();
        match usvg::Tree::from_data(vec_u8, &usvg::Options::default().to_ref()) {
            Ok(_rtree) => {
                r.kind = IconKind::Svg;
            }
            Err(e) => {
                r.message = format!("not_svg: {}", e);
            }
        }
        r
    }
}

struct InvWebp {}
impl InvestigateOne for InvWebp {
    fn investigate(&self, vec_u8: &Vec<u8>) -> IconAnalyseResult {
        let mut r = IconAnalyseResult::default();
        match libwebp_image::webp_load_from_memory(vec_u8) {
            Ok(_rtree) => {
                r.kind = IconKind::Webp;
            }
            Err(e) => {
                r.message = format!("not_webp: {}", e);
            }
        }
        r
    }
}

struct InvBmp {}
impl InvestigateOne for InvBmp {
    fn investigate(&self, vec_u8: &Vec<u8>) -> IconAnalyseResult {
        let mut r = IconAnalyseResult::default();
        match tinybmp::RawBmp::from_slice(vec_u8) {
            Ok(_decoder) => {
                r.kind = IconKind::Bmp;
            }
            Err(e) => {
                r.message = format!("not_bmp: {:?}", e);
            }
        }
        r
    }
}

#[cfg(test)]
mod t_ {
    use super::*;
    use crate::web::mockfilefetcher;

    //RUST_BACKTRACE=1 cargo watch -s "cargo test  downloader::icons::t_::t_analyze_icon  --lib -- --exact --nocapture "
    #[test]
    fn t_analyze_icon() {
        let set: [(&str, IconKind); 8] = [
            ("tests/data/favicon.ico", IconKind::Ico),          //
            ("tests/data/icon_651.ico", IconKind::Png),         //
            ("tests/data/report24-favicon.ico", IconKind::Ico), // is jpg
            ("tests/data/naturalnews_favicon.ico", IconKind::Ico),
            ("tests/data/heise-safari-pinned-tab.svg", IconKind::Svg),
            ("tests/data/gorillavsbear_townsquare.ico", IconKind::Ico), // MS Windows icon resource - 3 icons, 48x48, 32 bits/pixel, 48x48, 32 bits/pixel
            ("tests/data/LHNN-Logo-Main-Color-1.png", IconKind::Png),
            (
                "tests/data/feeds_seoulnews_favicon.ico",
                IconKind::UnknownType,
            ),
        ];
        set.iter().for_each(|(s, e_kind)| {
            let blob = mockfilefetcher::file_to_bin(s).unwrap();
            let r = icon_analyser(&blob);
            // println!(                "TEST  {} \t {:?}\t{}  {}x{} ",                s, r.kind, r.message, r.width_orig, r.heigth_orig            );
            assert_eq!(r.kind, *e_kind);
        });
    }

    /*
        #[allow(dead_code)]
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

    */
}
