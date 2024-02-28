use crate::controller::sourcetree::SJob;
use crate::db::icon_repo::IconEntry;
use crate::db::icon_repo::IconRepo;
use crate::downloader::util;
use crate::util::downscale_image;
use crate::util::Step;
use crate::util::StepResult;
use crate::web::WebFetcherType;
use flume::Sender;

use super::icons::decide_downscale;
use super::icons::icon_analyser;
use super::icons::ICON_CONVERT_TO_WIDTH;

/// for new-source dialog
pub struct ComprehensiveInner {
    pub feed_url_edit: String,
    pub url_download_text: String,
    pub feed_title: String,
    pub feed_homepage: String,
    pub icon_url: String,
    pub icon_bytes: Vec<u8>,
    pub iconrepo: IconRepo,
    pub web_fetcher: WebFetcherType,
    pub download_error_happened: bool,
    pub sourcetree_job_sender: Sender<SJob>,
    pub icon_id: isize,
}

impl std::fmt::Debug for ComprehensiveInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("")
            .field("url", &self.feed_url_edit)
            .field("title", &self.feed_title)
            .field("homepage", &self.feed_homepage)
            .field("icon_url", &self.icon_url)
            .field("E", &self.download_error_happened)
            .finish()
    }
}

impl PartialEq for ComprehensiveInner {
    fn eq(&self, other: &Self) -> bool {
        self.feed_url_edit == other.feed_url_edit
    }
}

pub struct ComprStart(ComprehensiveInner);
impl ComprStart {
    pub fn new(i: ComprehensiveInner) -> Self {
        ComprStart(i)
    }
}
impl Step<ComprehensiveInner> for ComprStart {
    fn step(self: Box<Self>) -> StepResult<ComprehensiveInner> {
        let mut inner: ComprehensiveInner = self.0;
        let url = inner.feed_url_edit.clone();
        let result = (*inner.web_fetcher).request_url(url.clone());
        match result.status {
            200 => {
                inner.url_download_text = result.content;
                StepResult::Continue(Box::new(ParseFeedString(inner)))
            }
            _ => {
                inner.download_error_happened = true;
                trace!(
                    "Feed download:  '{}' => {} {} {:?}",
                    &url,
                    result.get_status(),
                    result.get_kind(),
                    result.error_description
                );
                StepResult::Continue(Box::new(ComprFinal(inner)))
            }
        }
    }
}

pub struct ParseFeedString(ComprehensiveInner);
impl Step<ComprehensiveInner> for ParseFeedString {
    fn step(self: Box<Self>) -> StepResult<ComprehensiveInner> {
        let mut inner: ComprehensiveInner = self.0;
        let (homepage, feed_title, _err_msg) = util::retrieve_homepage_from_feed_text(
            inner.url_download_text.as_bytes(),
            &inner.feed_url_edit,
        );
        if !homepage.is_empty() {
            inner.feed_homepage = homepage;
        }
        if !feed_title.is_empty() {
            inner.feed_title = feed_title;
        }
        trace!(
            "COMPR2:  HP={}  TI={}",
            inner.feed_homepage,
            inner.feed_title
        );
        if !inner.feed_homepage.is_empty() {
            StepResult::Continue(Box::new(ComprAnalyzeHomepage(inner)))
        } else {
            StepResult::Continue(Box::new(ComprLoadIcon(inner)))
        }
    }
}

pub struct ComprAnalyzeHomepage(ComprehensiveInner);
impl Step<ComprehensiveInner> for ComprAnalyzeHomepage {
    fn step(self: Box<Self>) -> StepResult<ComprehensiveInner> {
        let mut inner: ComprehensiveInner = self.0;
        // debug!("ComprAnalyzeHomepage: {}   icon_url={}", &inner.feed_homepage   , inner.icon_url );
        let r = (*inner.web_fetcher).request_url(inner.feed_homepage.clone());
        match r.status {
            200 => match util::extract_icon_from_homepage(r.content, &inner.feed_homepage) {
                Ok(icon_url) => {
                    inner.icon_url = icon_url;
                }
                Err(descr) => {
                    debug!("XI: {} {}", inner.feed_homepage, descr);
                }
            },
            _ => {
                debug!(
                    "ComprAnalyzeHomepage: {:?} {}",
                    r.status, r.error_description
                );
            }
        }
        StepResult::Continue(Box::new(ComprLoadIcon(inner)))
    }
}

pub struct ComprLoadIcon(ComprehensiveInner);
impl Step<ComprehensiveInner> for ComprLoadIcon {
    fn step(self: Box<Self>) -> StepResult<ComprehensiveInner> {
        let mut inner: ComprehensiveInner = self.0;
        if inner.icon_url.is_empty() {
            if inner.feed_homepage.is_empty() {
                inner.icon_url = util::feed_url_to_icon_url(inner.feed_url_edit.clone());
            } else {
                inner.icon_url = util::feed_url_to_icon_url(inner.feed_homepage.clone());
            }
        }
        if inner.icon_url.is_empty() {
            return StepResult::Continue(Box::new(ComprFinal(inner)));
        }
        let r = (*inner.web_fetcher).request_url_bin(inner.icon_url.clone());
        match r.status {
            200 => {
                trace!(
                    "icon-download: {} '{}'  =>  {} {} {} ",
                    inner.feed_url_edit,
                    inner.icon_url,
                    &r.get_status(),
                    r.get_kind(),
                    r.error_description
                );
                inner.icon_bytes = r.content_bin;

                StepResult::Continue(Box::new(ComprStoreIcon(inner)))
            }
            _ => {
                inner.download_error_happened = true;
                debug!(
                    "icon-download: {} '{}'  =>  {} {} {} ",
                    inner.feed_url_edit,
                    inner.icon_url,
                    r.get_status(),
                    r.get_kind(),
                    r.error_description
                );
                StepResult::Continue(Box::new(ComprFinal(inner)))
            }
        }
    }
}

pub struct ComprStoreIcon(ComprehensiveInner);
impl Step<ComprehensiveInner> for ComprStoreIcon {
    fn step(self: Box<Self>) -> StepResult<ComprehensiveInner> {
        let mut inner: ComprehensiveInner = self.0;
        if inner.icon_bytes.len() < 10 {
            debug!(
                "ComprStoreIcon: icon too small: {} {}",
                inner.icon_url, inner.feed_url_edit
            );
            return StepResult::Continue(Box::new(ComprFinal(inner)));
        }

        let an_res = icon_analyser(&inner.icon_bytes);

        if decide_downscale(inner.icon_bytes.len(), &an_res) {
            match downscale_image(&inner.icon_bytes, &an_res.kind, ICON_CONVERT_TO_WIDTH) {
                Ok(r) => {
                    debug!("ComprStoreIcon: downscaled {:?}  ", inner.icon_url);
                    inner.icon_bytes = r;
                }
                Err(e) => {
                    debug!("downscale {:?} error {:?} ", inner.icon_url, e);
                }
            }
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
                    trace!("compr: stored icon {} {} ", entry.icon_id, inner.icon_url);
                    inner.icon_id = entry.icon_id;
                }
                Err(e) => {
                    warn!("Storing Icon from {}  failed {:?} ", inner.icon_url, e);
                }
            }
        } else {
            inner.icon_id = existing_icons[0].icon_id;
        }
        StepResult::Continue(Box::new(ComprFinal(inner)))
    }
}

pub struct ComprFinal(ComprehensiveInner);
impl Step<ComprehensiveInner> for ComprFinal {
    fn step(self: Box<Self>) -> StepResult<ComprehensiveInner> {
        let inner: ComprehensiveInner = self.0;
        let _r = inner.sourcetree_job_sender.send(SJob::NewFeedSourceEdit(
            inner.feed_url_edit.clone(),
            inner.feed_title.clone(),
            inner.icon_id,
            inner.feed_homepage.clone(),
        ));
        StepResult::Stop(inner)
    }
}
