use crate::controller::sourcetree::SJob;
use crate::db::errorentry::ESRC;
use crate::db::errors_repo::ErrorRepo;
use crate::db::icon_repo::IIconRepo;
use crate::db::icon_repo::IconRepo;
use crate::db::icon_row::CompressionType;
use crate::db::icon_row::IconRow;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::downloader::util;
use crate::util::db_time_to_display;
use crate::util::downscale_image;
use crate::util::png_from_svg;
use crate::util::IconKind;
use crate::util::Step;
use crate::util::StepResult;
use crate::web::mockfilefetcher::FileFetcher;
use crate::web::HttpGetResult;
use crate::web::WebFetcherType;
use flume::Receiver;
use flume::Sender;
use ico::ResourceType;
use resources::parameter::ICON_SIZE_LIMIT_BYTES;
use std::sync::Arc;
use std::time::Instant;

pub const ICON_CONVERT_TO_WIDTH: u32 = 48;

pub const ICON_WARNING_SIZE_BYTES: usize = 20000;

pub struct IconInner {
    pub subs_id: isize,

    /// TODO : check if needed!
    // #[deprecated]
    pub fs_icon_id_old: isize,

    pub feed_url: String,
    pub icon_url: String,
    pub iconrepo: IconRepo,
    pub icon_kind: IconKind,
    pub web_fetcher: WebFetcherType,
    pub download_error_happened: bool,
    pub sourcetree_job_sender: Sender<SJob>,
    pub feed_homepage: String,
    pub feed_download_text: String,
    pub subscriptionrepo: SubscriptionRepo,
    pub erro_repo: ErrorRepo,
    pub compressed_icon: String,

    pub dl_icon_bytes: Vec<u8>,
    pub dl_datetime_stamp: i64,
    /// Server sided size
    pub dl_icon_size: i64,
    // icon-id  retrieved from database
    pub db_icon_id: isize,
}

impl IconInner {
    pub fn new_in_mem(
        filefetcher_base: &str,
        subscription_id: isize,
    ) -> (IconInner, Receiver<SJob>) {
        let (stc_job_s, _stc_job_r) = flume::bounded::<SJob>(1);
        let ff = FileFetcher::new(filefetcher_base.to_string());
        let fetcher_a: WebFetcherType = Arc::new(Box::new(ff));
        let mut icon_inner = IconInner::new(
            stc_job_s,
            fetcher_a,
            IconRepo::new_in_mem(),
            SubscriptionRepo::new_inmem(),
            ErrorRepo::new_in_mem(),
        );
        /*
         {
            subs_id: 1,
            feed_url: String::default(),
            iconrepo: IconRepo::new_in_mem(),
            web_fetcher: fetcher_a,
            download_error_happened: false,F
            icon_url: String::default(),
            fs_icon_id_old: 0,
            sourcetree_job_sender: stc_job_s,
            feed_homepage: String::default(),
            feed_download_text: String::default(),
            subscriptionrepo: SubscriptionRepo::new_inmem(),
            erro_repo: ErrorRepo::new_in_mem(),
            icon_kind: Default::default(),
            compressed_icon: Default::default(),
            dl_icon_bytes: Default::default(),
            dl_datetime_stamp: 0,
            dl_icon_size: 0,
            db_icon_id: -1,
        };
         */
        icon_inner.subs_id = subscription_id;
        icon_inner.subscriptionrepo.scrub_all_subscriptions();
        (icon_inner, _stc_job_r)
    }

    pub fn new(
        sourcetree_job_sender: Sender<SJob>,
        web_fetche: WebFetcherType,
        iconrep: IconRepo,
        subscriptionrepo: SubscriptionRepo,
        errorrepo: ErrorRepo,
    ) -> IconInner {
        // let (stc_job_s, stc_job_r) = flume::bounded::<SJob>(1);

        let icon_inner = IconInner {
            sourcetree_job_sender: sourcetree_job_sender, // stc_job_s,
            subscriptionrepo: subscriptionrepo,
            erro_repo: errorrepo,
            web_fetcher: web_fetche,
            subs_id: -1,
            feed_url: String::default(),
            iconrepo: iconrep,
            download_error_happened: false,
            icon_url: String::default(),
            fs_icon_id_old: -1,
            feed_homepage: String::default(),
            feed_download_text: String::default(),
            icon_kind: Default::default(),
            compressed_icon: Default::default(),
            dl_icon_bytes: Default::default(),
            dl_datetime_stamp: 0,
            dl_icon_size: -1,
            db_icon_id: -1,
        };
        icon_inner
    }
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
        if inner.subs_id < 0 {
            error!("IconLoadStart:  subs_id:{}  but must be >0!", inner.subs_id);
            return StepResult::Stop(inner);
        }

        if let Some(subs_e) = inner.subscriptionrepo.get_by_index(inner.subs_id) {
            if !inner.icon_url.is_empty() {
                trace!("IconLoadStart:  ID:{}  db-HP:{} prev-icon-url:{}  HP:{} icon_id:{}  {}  feed-url:{} ",
                  inner.subs_id,  subs_e.website_url, inner.icon_url, inner.feed_homepage,
                  subs_e.icon_id,subs_e.display_name, subs_e.url );
            }
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
        // trace!("FeedTextDownload:   feed-url:{} ", inner.feed_url);
        let result = (*inner.web_fetcher).request_url(&inner.feed_url);
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
        let dl_text = util::workaround_https_declaration(&inner.feed_download_text);
        let (homepage, _feed_title, errtext) =
            util::retrieve_homepage_from_feed_text(dl_text.as_bytes(), &inner.feed_url);
        // trace!( "HomepageDownload({})  i_hp:{}  retr_hp:{}  title:{}  err:{}    feed_url:{} ",            inner.subs_id,            inner.feed_homepage,            homepage,            _feed_title,            errtext,            inner.feed_url        );
        if !homepage.is_empty() {
            if homepage != inner.feed_url {
                inner.feed_homepage = homepage;
            } else {
                if inner.feed_url.is_empty() {
                    warn!("NO feed_url:{}   HP:", inner.feed_homepage);
                }
                let alt_hp = util::feed_url_to_main_url(inner.feed_url.clone());
                inner.feed_homepage = alt_hp;
            }
            return StepResult::Continue(Box::new(CompareHomepageToDB(inner)));
        } else {
            debug!(
                "got no HP  from feed text!  Feed-URL: {}   {}",
                &inner.feed_url, errtext
            );
            inner.erro_repo.add_error(
                inner.subs_id,
                ESRC::IconNoHomepageFromFeedtext,
                0,
                inner.feed_url.clone(),
                errtext,
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
        let homepage: String = inner.feed_homepage.clone();
        // trace!(            "IconAnalyzeHomepage({})   feed_hp:{} ",            inner.subs_id,            homepage        );
        let r = (*inner.web_fetcher).request_url(&homepage);
        match r.status {
            200 | 202 => match util::extract_icon_from_homepage(r.content, &homepage) {
                Ok(icon_url) => {
                    inner.icon_url = icon_url;
                    return StepResult::Continue(Box::new(IconDownload(inner)));
                }
                Err(e_descr) => {
                    // trace!(                        "IconAnalyzeHomepage({}) E: {}  {} ",                        inner.subs_id,                        e_descr,                        homepage                    );
                    inner.erro_repo.add_error(
                        inner.subs_id,
                        ESRC::IconsAHEx,
                        r.status as isize,
                        homepage,
                        e_descr,
                    );
                }
            },
            _ => {
                let alt_hp = util::feed_url_to_main_url(inner.feed_url.clone());
                // trace!(                    "IconAnalyzeHomepage({})   STATUS:{}  alt_hp:{} ",                    inner.subs_id,                    r.status,                    alt_hp                );
                inner.erro_repo.add_error(
                    inner.subs_id,
                    ESRC::IconsAHMain,
                    r.status as isize,
                    homepage,
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
        // trace!("IconDownload({}) {} ", inner.subs_id, &inner.icon_url);
        let now = Instant::now();
        let r: HttpGetResult = (*inner.web_fetcher).request_url_bin(&inner.icon_url);
        let elapsedms = now.elapsed().as_millis();
        match r.status {
            200 => {
                inner.dl_icon_bytes = r.content_bin;
                inner.dl_icon_size = r.content_length;
                inner.dl_datetime_stamp = r.timestamp;
                if r.content_length <= 1 {
                    // info!(                        "IconDownload({}) {} content-length:{} num-bytes:{} ",                        inner.subs_id,                        &inner.icon_url,                        r.content_length,                        inner.dl_icon_bytes.len()                    );
                    inner.dl_icon_size = inner.dl_icon_bytes.len() as i64;
                }
                if elapsedms > 100 {
                    inner.erro_repo.add_error(
                        inner.subs_id,
                        ESRC::IconDLDuration,
                        elapsedms as isize,
                        inner.icon_url.to_string(),
                        String::default(),
                    );
                }
                StepResult::Continue(Box::new(IconIsInDatabase(inner)))
            }
            _ => {
                inner.download_error_happened = true;
                if r.get_status() != 404 {
                    trace!(
                        "IconDownload ERR {} {}  {}   {}   ",
                        r.get_status(),
                        r.error_description,
                        r.get_kind(),
                        inner.icon_url
                    );
                }
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

pub struct IconIsInDatabase(pub IconInner);
impl Step<IconInner> for IconIsInDatabase {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;
        if inner.dl_icon_bytes.len() < 10 {
            trace!(
                "downloaded icon_too_small! {:?} {:?}",
                inner.dl_icon_bytes,
                inner.icon_url
            );
            return StepResult::Stop(inner);
        }
        if inner.dl_icon_bytes.len() > ICON_WARNING_SIZE_BYTES {
            trace!(
                "IconIsInDatabase: {} big size: {} kB",
                inner.icon_url,
                inner.dl_icon_bytes.len() / 1024
            );
        }

        let icons_in_db: Vec<IconRow> = inner.iconrepo.get_by_web_url(&inner.icon_url);
        if icons_in_db.len() > 1 {
            debug!(
                "IconIsInDatabase({}) {}   multiple icons for that url:{} ",
                inner.subs_id,
                &inner.icon_url,
                icons_in_db.len()
            );
        }
        if icons_in_db.len() == 0 {
            return StepResult::Continue(Box::new(IconCheckIsImage(inner)));
        }
        let icon_per_url: &IconRow = icons_in_db.get(0).unwrap();
        if icon_per_url.web_date == inner.dl_datetime_stamp {
            // trace!("IconPerUrl {} same timestamp, good", &inner.icon_url);

            inner.db_icon_id = icon_per_url.icon_id;
            return StepResult::Continue(Box::new(UseIconForDisplay(inner)));
        }

        trace!(
            "IconPerUrl {} different timestamp,  db:{} {}bytes    web:{} {}bytes  processing... ",
            &inner.icon_url,
            db_time_to_display(icon_per_url.web_date),
            icon_per_url.web_size,
            db_time_to_display(inner.dl_datetime_stamp),
            inner.dl_icon_size
        );

        //         info!(            "IconPerUrl  db_date:{}  !=  web_date:{}  TODO",            icon_per_url.web_date, inner.dl_datetime_stamp        );

        StepResult::Continue(Box::new(IconCheckIsImage(inner)))
    }
}

pub struct IconCheckIsImage(pub IconInner);
impl Step<IconInner> for IconCheckIsImage {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;
        let an_res: IconAnalyseResult = icon_analyser(&inner.dl_icon_bytes);
        inner.icon_kind = an_res.kind.clone();
        trace!(
            "IconCheckIsImage:  Kind {:?}  {}  {} disguised_as_png={} ",
            an_res.kind,
            inner.feed_homepage,
            inner.icon_url,
            an_res.icon_disguised_as_png
        );
        if an_res.kind == IconKind::Svg {
            return StepResult::Continue(Box::new(IconSvgToPng(inner)));
        }
        if decide_downscale(inner.dl_icon_bytes.len(), &an_res) {
            // trace!(                "IconCheckIsImage:  go to downscale  Url:{} ",               inner.icon_url            );
            return StepResult::Continue(Box::new(IconDownscale(inner)));
        }
        if an_res.kind == IconKind::AnalyseDoneUnknown || an_res.kind == IconKind::TooSmall {
            inner.erro_repo.add_error(
                inner.subs_id,
                ESRC::IconsCheckimg,
                inner.dl_icon_bytes.len() as isize,
                inner.icon_url.clone(),
                an_res.message,
            );
            return StepResult::Stop(inner);
        }
        StepResult::Continue(Box::new(IconCheckPresent(inner)))
    }
}

// solution for compressed svg:   downscale them before
pub fn decide_downscale(length: usize, an_res: &IconAnalyseResult) -> bool {
    (an_res.width_orig > ICON_CONVERT_TO_WIDTH
        || an_res.height_orig > ICON_CONVERT_TO_WIDTH
        || length > ICON_SIZE_LIMIT_BYTES
        || an_res.kind == IconKind::Webp
        || an_res.icon_disguised_as_png)
        && an_res.kind != IconKind::AnalyseDoneUnknown
}

pub struct IconSvgToPng(pub IconInner);
impl Step<IconInner> for IconSvgToPng {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;
        let r = png_from_svg(&inner.dl_icon_bytes);
        if r.is_err() {
            let msg = format!(
                "SvgToPng:{:?} {} {} {:?}",
                &inner.icon_kind,
                inner.feed_url,
                inner.icon_url,
                r.err()
            );
            debug!("{msg}");
            inner.erro_repo.add_error(
                inner.subs_id,
                ESRC::IconsSvgToPng,
                0,
                inner.icon_url.clone(),
                msg,
            );
            return StepResult::Stop(inner);
        }
        inner.dl_icon_bytes = r.unwrap();
        inner.icon_kind = IconKind::Png;
        StepResult::Continue(Box::new(IconDownscale(inner)))
    }
}

pub struct IconDownscale(pub IconInner);
impl Step<IconInner> for IconDownscale {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;
        // trace!("IconDownscale: ... {:?} ", inner.icon_kind);
        let r = downscale_image(
            &inner.dl_icon_bytes,
            &inner.icon_kind,
            ICON_CONVERT_TO_WIDTH,
        );
        if r.is_err() {
            let msg = format!(
                "downscale:{:?} {} {} {:?}",
                &inner.icon_kind,
                inner.feed_url,
                inner.icon_url,
                r.err()
            );
            // trace!("{msg}");
            inner.erro_repo.add_error(
                inner.subs_id,
                ESRC::IconsDownscale,
                0,
                inner.icon_url.clone(),
                msg,
            );
            return StepResult::Stop(inner);
        }
        inner.dl_icon_bytes = r.unwrap();
        StepResult::Continue(Box::new(IconCheckPresent(inner)))
    }
}

pub struct IconCheckPresent(pub IconInner);
impl Step<IconInner> for IconCheckPresent {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;

        // TODO  ....  is it in DB ??  necessary??

        inner.compressed_icon = util::compress_vec_to_string(&inner.dl_icon_bytes);
        let existing_icons: Vec<IconRow> =
            inner.iconrepo.get_by_icon(inner.compressed_icon.clone());

        if !existing_icons.is_empty() {
            let existing_id = existing_icons[0].icon_id;
            trace!(
                "icon already in DB: {}=>{}  subs:{}  U:{} ",
                inner.fs_icon_id_old,
                existing_id,
                inner.subs_id,
                inner.icon_url
            );
            if existing_id != inner.fs_icon_id_old {
                // TODO  ....  is it in DB ??  necessary??
                return StepResult::Continue(Box::new(UseIconForDisplay(inner)));
            }
        }
        StepResult::Continue(Box::new(IconStore(inner)))
    }
}

// Later:   utilize    http_date, http_length
struct IconStore(IconInner);
impl Step<IconInner> for IconStore {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let mut inner: IconInner = self.0;
        assert!(!inner.compressed_icon.is_empty());
        match inner.iconrepo.add_icon(
            inner.compressed_icon.clone(),
            inner.dl_datetime_stamp,
            inner.dl_icon_size as isize,
            inner.icon_url.clone(),
            CompressionType::ImageRs,
        ) {
            Ok(icon_id) => {
                debug!( "IconStore:  Web-len:{}  compr-len:{:?}  => ID {}  F:{}  HP:{}  Web-Last-Mod:{} --> SetIconId" ,
                        inner.dl_icon_size, inner.compressed_icon.len(),  icon_id, inner.feed_url,  inner.feed_homepage,  db_time_to_display (inner.dl_datetime_stamp)   );
                inner.db_icon_id = icon_id as isize;
                return StepResult::Continue(Box::new(UseIconForDisplay(inner)));
            }
            Err(e) => {
                error!("Storing Icon from {}  failed {:?}", inner.icon_url, e);
            }
        }
        StepResult::Stop(inner)
    }
}

struct UseIconForDisplay(IconInner);
impl Step<IconInner> for UseIconForDisplay {
    fn step(self: Box<Self>) -> StepResult<IconInner> {
        let _r = self
            .0
            .sourcetree_job_sender
            .send(SJob::SetIconId(self.0.subs_id, self.0.db_icon_id as isize));
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

#[derive(Debug, Default)]
pub struct IconAnalyseResult {
    pub kind: IconKind,
    width_orig: u32,
    height_orig: u32,
    pub message: String,
    icon_disguised_as_png: bool,
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

// there are icons that are named .ico but that are png.  Try png first
pub fn icon_analyser(vec_u8: &[u8]) -> IconAnalyseResult {
    let analysers: [Box<dyn InvestigateOne>; 8] = [
        Box::new(BySize {}),
        Box::new(InvJpg {}),
        Box::new(InvPng {}),
        Box::new(InvIco {}),
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
    IconAnalyseResult::with_msg(IconKind::AnalyseDoneUnknown, msgs.join(" "))
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
                if decoder.resource_type() != ResourceType::Icon {
                    debug!("InvIco:  not handled  {:?} ", decoder.resource_type());
                }
                if let Some(entry) = decoder.entries().first() {
                    // trace!(                        "InvIco: E0:  isPng:{}  BpP:{} ",                        entry.is_png(),                        entry.bits_per_pixel()                    );
                    r.width_orig = entry.width();
                    r.height_orig = entry.height();
                    if entry.is_png() {
                        r.icon_disguised_as_png = true;
                    }
                }
            }
            Err(e) => {
                r.message = format!("not_ico: {e}");
                // debug!("InvIco:  not_ico {} ", r.message);
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
                // debug!("InvPng:    Not-png  {} ", r.message);
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
        let mut cursor: std::io::Cursor<&[u8]> = std::io::Cursor::new(vec_u8);
        match bmp::from_reader(&mut cursor) {
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
