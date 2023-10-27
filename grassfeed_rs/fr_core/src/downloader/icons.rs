use crate::controller::sourcetree::SJob;
use crate::db::errorentry::ESRC;
use crate::db::errors_repo::ErrorRepo;
use crate::db::icon_repo::IconEntry;
use crate::db::icon_repo::IconRepo;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::downloader::util;
use crate::downloader::util::workaround_https_declaration;
use crate::util::downscale_image;
use crate::util::IconKind;
use crate::util::Step;
use crate::util::StepResult;
use crate::web::WebFetcherType;
use flume::Sender;
use resources::parameter::ICON_SIZE_LIMIT_BYTES;
use std::time::Instant;

pub const ICON_CONVERT_TO_WIDTH: u32 = 48;

pub struct IconInner {
    pub subs_id: isize,
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
    pub image_icon_kind: IconKind,
    pub compressed_icon: String,
}

impl std::fmt::Debug for IconInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("")
            .field("subs_id", &self.subs_id)
            .field("feed", &self.feed_url)
            .finish()
    }
}

impl PartialEq for IconInner {
    fn eq(&self, other: &Self) -> bool {
        self.subs_id == other.subs_id
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
        if let Some(subs_e) = inner.subscriptionrepo.get_by_index(inner.subs_id) {
            // if !inner.icon_url.is_empty() {                 trace!(                    "IconLoadStart: db-HP:{}   prev-iconurl:{}",                    subs_e.website_url,                    inner.icon_url                );            }
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
        let now = Instant::now();
        let result = (*inner.web_fetcher).request_url(inner.feed_url.clone());
        let elapsedms = now.elapsed().as_millis();
        match result.status {
            200 => {
                if elapsedms > 100 {
                    inner.erro_repo.add_error(
                        inner.subs_id,
                        ESRC::IconFeedTextDur,
                        elapsedms as isize,
                        inner.feed_url.to_string(),
                        String::default(),
                    );
                }
                inner.feed_download_text = result.content;
            }
            _ => {
                inner.erro_repo.add_error(
                    inner.subs_id,
                    ESRC::IconsFeedtext,
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
        let dl_text = workaround_https_declaration(&inner.feed_download_text);
        let (homepage, _feed_title, errtext) =
            util::retrieve_homepage_from_feed_text(dl_text.as_bytes(), &inner.feed_url);
        if !homepage.is_empty() {
            if homepage != inner.feed_url {
                inner.feed_homepage = homepage;
            } else {
                let alt_hp = util::feed_url_to_main_url(inner.feed_url.clone());
                // debug!("found_HP==feed-url :-/ ALT-HP={}", alt_hp);
                inner.feed_homepage = alt_hp;
            }
            return StepResult::Continue(Box::new(CompareHomepageToDB(inner)));
        } else {
            trace!(
                "got no HP  from feed text!  Feed-URL: {}   {}",
                &inner.feed_url,
                errtext
            );
        }
        StepResult::Continue(Box::new(IconFallbackSimple(inner)))
    }
}

struct CompareHomepageToDB(IconInner);
impl Step<IconInner> for CompareHomepageToDB {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let inner: IconInner = self.0;
        if let Some(subs_e) = inner.subscriptionrepo.get_by_index(inner.subs_id) {
            if !inner.feed_homepage.is_empty() && inner.feed_homepage != subs_e.website_url {
                inner
                    .subscriptionrepo
                    .update_homepage(inner.subs_id, &inner.feed_homepage);
            }
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
            200 => match util::extract_icon_from_homepage(r.content, &inner.feed_homepage) {
                Ok(icon_url) => {
                    inner.icon_url = icon_url;
                    trace!(
                        "IconAnalyzeHomepage( {} ) : iconurl {} ",
                        inner.subs_id,
                        &inner.icon_url
                    );
                    return StepResult::Continue(Box::new(IconDownload(inner)));
                }
                Err(e_descr) => {
                    inner.erro_repo.add_error(
                        inner.subs_id,
                        ESRC::IconsAHEx,
                        r.status as isize,
                        inner.feed_homepage.clone(),
                        e_descr,
                    );
                }
            },
            _ => {
                let alt_hp = util::feed_url_to_main_url(inner.feed_url.clone());
                inner.erro_repo.add_error(
                    inner.subs_id,
                    ESRC::IconsAHMain,
                    r.status as isize,
                    inner.feed_homepage.clone(),
                    r.error_description,
                );
                if inner.feed_homepage != alt_hp {
                    inner.feed_homepage = alt_hp;
                    return StepResult::Continue(Box::new(IconAnalyzeHomepage(inner)));
                }
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
        let now = Instant::now();
        let r = (*inner.web_fetcher).request_url_bin(inner.icon_url.clone());
        let elapsedms = now.elapsed().as_millis();
        match r.status {
            200 => {
                inner.icon_bytes = r.content_bin;
                if elapsedms > 100 {
                    inner.erro_repo.add_error(
                        inner.subs_id,
                        ESRC::IconDLDuration,
                        elapsedms as isize,
                        inner.icon_url.to_string(),
                        String::default(),
                    );
                }
                StepResult::Continue(Box::new(IconCheckIsImage(inner)))
            }
            _ => {
                inner.download_error_happened = true;
                inner.erro_repo.add_error(
                    inner.subs_id,
                    ESRC::IconsDownload,
                    r.get_status() as isize,
                    inner.icon_url.clone(),
                    format!("IconDownload K:{}  {}", r.get_kind(), r.error_description),
                );
                StepResult::Stop(inner)
            }
        }
    }
}

pub struct IconCheckIsImage(pub IconInner);
impl Step<IconInner> for IconCheckIsImage {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;
        let an_res = icon_analyser(&inner.icon_bytes);
        inner.image_icon_kind = an_res.kind.clone();
        if (an_res.width_orig > ICON_CONVERT_TO_WIDTH
            || an_res.height_orig > ICON_CONVERT_TO_WIDTH
            || inner.icon_bytes.len() > ICON_SIZE_LIMIT_BYTES
            || an_res.kind == IconKind::Webp)
            && an_res.kind != IconKind::UnknownType
        {
            return StepResult::Continue(Box::new(IconDownscale(inner)));
        }
        if an_res.kind == IconKind::UnknownType || an_res.kind == IconKind::TooSmall {
            inner.erro_repo.add_error(
                inner.subs_id,
                ESRC::IconsCheckimg,
                inner.icon_bytes.len() as isize,
                inner.icon_url.clone(),
                an_res.message,
            );
            return StepResult::Stop(inner);
        }
        StepResult::Continue(Box::new(IconCheckPresent(inner)))
    }
}

pub struct IconDownscale(pub IconInner);
impl Step<IconInner> for IconDownscale {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;
        let r = downscale_image(
            &inner.icon_bytes,
            &inner.image_icon_kind,
            ICON_CONVERT_TO_WIDTH,
        );
        if r.is_err() {
            let msg = format!(
                "downscale:{:?} {} {} {:?}",
                &inner.image_icon_kind,
                inner.feed_url,
                inner.icon_url,
                r.err()
            );
            trace!("{msg}");
            inner.erro_repo.add_error(
                inner.subs_id,
                ESRC::IconsDownscale,
                0,
                inner.icon_url.clone(),
                msg,
            );
            return StepResult::Stop(inner);
        }
        inner.icon_bytes = r.unwrap();
        StepResult::Continue(Box::new(IconCheckPresent(inner)))
    }
}

struct IconCheckPresent(IconInner);
impl Step<IconInner> for IconCheckPresent {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;

        // trace!(            "IconCheckPresent( {} ) url {}  len {} ",            inner.subs_id,            inner.icon_url,            inner.icon_bytes.len()        );

        if inner.icon_bytes.len() < 10 {
            error!(
                "downloaded icon_too_small! {:?} {:?}",
                inner.icon_bytes, inner.icon_url
            );
            return StepResult::Stop(inner);
        }
        if inner.icon_bytes.len() > 2000 {
            debug!(
                "IconCheckPresent: {} {} \t big size: {} kB",
                inner.icon_url,
                inner.feed_url,
                inner.icon_bytes.len() / 1024
            );
        }
        inner.compressed_icon = util::compress_vec_to_string(&inner.icon_bytes);
        let existing_icons: Vec<IconEntry> =
            inner.iconrepo.get_by_icon(inner.compressed_icon.clone());
        if !existing_icons.is_empty() {
            let existing_id = existing_icons[0].icon_id;
            //  trace!(                "icon already there. {}=>{}  subs: {} ",                inner.fs_icon_id_old,                existing_id,                inner.subs_id            );
            if existing_id != inner.fs_icon_id_old {
                let _r = inner
                    .sourcetree_job_sender
                    .send(SJob::SetIconId(inner.subs_id, existing_icons[0].icon_id));
                return StepResult::Stop(inner);
            }
        }
        StepResult::Continue(Box::new(IconStore(inner)))
    }
}

struct IconStore(IconInner);
impl Step<IconInner> for IconStore {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let inner: IconInner = self.0;

        assert!(!inner.compressed_icon.is_empty());
        let ie = IconEntry {
            icon: inner.compressed_icon.clone(),
            ..Default::default()
        };
        match inner.iconrepo.store_entry(&ie) {
            Ok(entry) => {
                debug!(
                    "IconStore:  len:{:?}  => ID {}  F:{}  HP:{} ",
                    ie.icon.len(),
                    entry.icon_id,
                    inner.feed_url,
                    inner.feed_homepage
                );
                let _r = inner
                    .sourcetree_job_sender
                    .send(SJob::SetIconId(inner.subs_id, entry.icon_id));
            }
            Err(e) => {
                error!("Storing Icon from {}  failed {:?}", inner.icon_url, e);
            }
        }
        StepResult::Stop(inner)
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

#[derive(Debug, Default)]
pub struct IconAnalyseResult {
    pub kind: IconKind,
    width_orig: u32,
    height_orig: u32,
    pub message: String,
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

pub fn icon_analyser(vec_u8: &[u8]) -> IconAnalyseResult {
    let analysers: [Box<dyn InvestigateOne>; 8] = [
        Box::new(BySize {}),
        Box::new(InvJpg {}),
        Box::new(InvIco {}),
        Box::new(InvPng {}),
        Box::new(InvGif {}),
        Box::new(InvWebp {}),
        Box::new(InvBmp {}),
        Box::new(InvSvg {}),
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
    fn investigate(&self, blob: &[u8]) -> IconAnalyseResult;
}

struct BySize {}
impl InvestigateOne for BySize {
    fn investigate(&self, vec_u8: &[u8]) -> IconAnalyseResult {
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
    fn investigate(&self, vec_u8: &[u8]) -> IconAnalyseResult {
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
                r.message = format!("not_ico: {e}");
            }
        }
        r
    }
}

struct InvPng {}
impl InvestigateOne for InvPng {
    fn investigate(&self, vec_u8: &[u8]) -> IconAnalyseResult {
        let mut r = IconAnalyseResult::default();
        let cursor = std::io::Cursor::new(vec_u8);
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
                r.message = format!("not_png: {e}");
            }
        }
        r
    }
}

struct InvJpg {}
impl InvestigateOne for InvJpg {
    fn investigate(&self, vec_u8: &[u8]) -> IconAnalyseResult {
        let mut r = IconAnalyseResult::default();
        let cursor = std::io::Cursor::new(vec_u8);
        let mut decoder = jpeg_decoder::Decoder::new(cursor);
        match decoder.decode() {
            Ok(_pixels) => {
                r.kind = IconKind::Jpg;
            }
            Err(e) => {
                r.message = format!("not_jpg: {e}");
            }
        }
        r
    }
}

struct InvGif {}
impl InvestigateOne for InvGif {
    #[cfg(feature = "legacy3gtk14")]
    fn investigate(&self, vec_u8: &[u8]) -> IconAnalyseResult {
        let mut r = IconAnalyseResult::default();
        let cursor = std::io::Cursor::new(vec_u8);
        let r_decoder = gif::Decoder::new(cursor);
        match r_decoder {
            Ok(mut decod) => {
                let r_frameinfo = decod.next_frame_info();
                if r_frameinfo.is_ok() {
                    r.kind = IconKind::Gif;
                }
            }
            Err(e) => {
                r.message = format!("not_gif: {e:?}");
            }
        }
        r
    }

    #[cfg(not(feature = "legacy3gtk14"))]
    fn investigate(&self, vec_u8: &[u8]) -> IconAnalyseResult {
        let mut r = IconAnalyseResult::default();
        let cursor = std::io::Cursor::new(vec_u8);

        let decoder = gif::DecodeOptions::new();
        match decoder.read_info(cursor) {
            Ok(mut decod2) => {
                let o_nextframe = decod2.read_next_frame();
                if o_nextframe.is_ok() {
                    r.kind = IconKind::Gif;
                }
            }
            Err(e) => {
                r.message = format!("not_gif: {e:?}");
            }
        }
        r
    }
}

struct InvSvg {}
impl InvestigateOne for InvSvg {
    #[cfg(not(feature = "legacy3gtk14"))]
    fn investigate(&self, vec_u8: &[u8]) -> IconAnalyseResult {
        let mut r = IconAnalyseResult::default();
        match usvg::Tree::from_data(vec_u8, &usvg::Options::default().to_ref()) {
            Ok(_rtree) => {
                r.kind = IconKind::Svg;
            }
            Err(e) => {
                r.message = format!("not_svg: {e:?}");
            }
        }
        r
    }

    #[cfg(feature = "legacy3gtk14")]
    fn investigate(&self, vec_u8: &[u8]) -> IconAnalyseResult {
        let mut r = IconAnalyseResult::default();
        match usvg::Tree::from_data(vec_u8, &usvg::Options::default()) {
            Ok(_rtree) => {
                r.kind = IconKind::Svg;
            }
            Err(e) => {
                r.message = format!("not_svg: {e:?}");
            }
        }
        r
    }
}

struct InvWebp {}
impl InvestigateOne for InvWebp {
    fn investigate(&self, vec_u8: &[u8]) -> IconAnalyseResult {
        let mut r = IconAnalyseResult::default();
        match libwebp_image::webp_load_from_memory(vec_u8) {
            Ok(_rtree) => {
                r.kind = IconKind::Webp;
            }
            Err(e) => {
                r.message = format!("not_webp: {e:?}");
            }
        }
        r
    }
}

struct InvBmp {}
impl InvestigateOne for InvBmp {
    fn investigate(&self, vec_u8: &[u8]) -> IconAnalyseResult {
        let mut r = IconAnalyseResult::default();
        match tinybmp::RawBmp::from_slice(vec_u8) {
            Ok(_decoder) => {
                r.kind = IconKind::Bmp;
            }
            Err(e) => {
                r.message = format!("not_bmp: {e:?}");
            }
        }
        r
    }
}
