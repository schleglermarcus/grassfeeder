use crate::controller::guiprocessor::Job;
use crate::controller::sourcetree::SJob;
use crate::db::errorentry::ESRC;
use crate::db::errors_repo::ErrorRepo;
use crate::downloader::util::extract_feed_from_website;
use crate::downloader::util::go_to_homepage;
use crate::util::Step;
use crate::util::StepResult;
use crate::web::WebFetcherType;
use feed_rs::parser;
use flume::Sender;

pub struct DragInner {
    pub web_fetcher: WebFetcherType,
    pub sourcetree_job_sender: Sender<SJob>,
    pub guiproc_job_sender: Sender<Job>,
    pub dragged_url: String,
    pub dragged_url_content: String,
    pub found_feed_url: String,
    pub found_homepage: String,
    pub error_message: String,
    pub erro_repo: ErrorRepo,
    pub testing_base_url: String,
    pub feed_display_title: String,
}

impl DragInner {
    pub fn new(
        drag_url: String,
        s_se: Sender<SJob>,
        w_fetcher: WebFetcherType,
        err_repo: ErrorRepo,
        gp_sender: Sender<Job>,
    ) -> Self {
        DragInner {
            web_fetcher: w_fetcher,
            sourcetree_job_sender: s_se,
            dragged_url: drag_url,
            erro_repo: err_repo,
            guiproc_job_sender: gp_sender,
            dragged_url_content: Default::default(),
            found_feed_url: Default::default(),
            found_homepage: Default::default(),
            error_message: Default::default(),
            testing_base_url: Default::default(),
            feed_display_title: Default::default(),
        }
    }
}

impl std::fmt::Debug for DragInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("").field("drag", &self.dragged_url).finish()
    }
}

impl PartialEq for DragInner {
    fn eq(&self, other: &Self) -> bool {
        self.dragged_url == other.dragged_url
    }
}

pub struct BrowserEvalStart(DragInner);
impl BrowserEvalStart {
    pub fn new(i: DragInner) -> Self {
        BrowserEvalStart(i)
    }
}

impl Step<DragInner> for BrowserEvalStart {
    fn step(self: Box<Self>) -> StepResult<DragInner> {
        let mut inner: DragInner = self.0;
        let result = (*inner.web_fetcher).request_url( & inner.dragged_url);
        if result.status == 200 {
            inner.dragged_url_content = result.content;
            return StepResult::Continue(Box::new(ParseWebpage(inner)));
        }
        inner.error_message = format!("{} {}", result.status, &result.error_description);
        inner.erro_repo.add_error(
            -1,
            ESRC::DragEvalstart,
            result.status as isize,
            inner.dragged_url.clone(),
            result.error_description,
        );
        StepResult::Continue(Box::new(CompleteRelativeUrl(inner)))
    }
}

struct ParseWebpage(DragInner);
impl Step<DragInner> for ParseWebpage {
    fn step(self: Box<Self>) -> StepResult<DragInner> {
        let mut inner: DragInner = self.0;
        let extr_r = extract_feed_from_website(&inner.dragged_url_content);
        if extr_r.is_err() {
            let (err_msg, _raw_txt) = extr_r.err().unwrap();
            inner.error_message = err_msg;
            return StepResult::Continue(Box::new(CheckContentIsFeed(inner)));
        }
        inner.found_feed_url = extr_r.unwrap();
        StepResult::Continue(Box::new(CompleteRelativeUrl(inner)))
    }
}

struct CheckContentIsFeed(DragInner);
impl Step<DragInner> for CheckContentIsFeed {
    fn step(self: Box<Self>) -> StepResult<DragInner> {
        let mut inner: DragInner = self.0;
        let parse_r = parser::parse(inner.dragged_url_content.as_bytes());
        if parse_r.is_err() {
            inner.error_message += &parse_r.err().unwrap().to_string();
            return StepResult::Continue(Box::new(AnalyzeContentSloppy(inner)));
        }
        let parsed = parse_r.unwrap();
        if let Some(t_t) = parsed.title {
            inner.feed_display_title = t_t.content;
        }
        inner.found_feed_url.clone_from(&inner.dragged_url);
        StepResult::Continue(Box::new(CompleteRelativeUrl(inner)))
    }
}

struct AnalyzeContentSloppy(DragInner);
impl Step<DragInner> for AnalyzeContentSloppy {
    fn step(self: Box<Self>) -> StepResult<DragInner> {
        let mut inner: DragInner = self.0;
        let extracted: Vec<String> = extract_feed_urls_sloppy(&inner.dragged_url_content);
        if !extracted.is_empty() {
            debug!("feed adresses found by sloppy:   {:?}", extracted);
            inner.found_feed_url.clone_from(extracted.first().unwrap());
        }
        StepResult::Continue(Box::new(CompleteRelativeUrl(inner)))
    }
}

struct CompleteRelativeUrl(DragInner);
impl Step<DragInner> for CompleteRelativeUrl {
    fn step(self: Box<Self>) -> StepResult<DragInner> {
        let mut inner: DragInner = self.0;
        let mut drag_url = inner.dragged_url.clone();
        if !drag_url.starts_with("http") {
            drag_url.clone_from(&inner.testing_base_url);
        }
        if !inner.found_feed_url.is_empty() && !inner.found_feed_url.starts_with("http") {
            let o_homepage_addr = go_to_homepage(&drag_url);
            if let Some(base_url) = o_homepage_addr {
                if !base_url.ends_with('/') && !inner.found_feed_url.starts_with('/') {
                    inner.found_feed_url = format!("{}/{}", base_url, inner.found_feed_url);
                } else {
                    inner.found_feed_url = format!("{}{}", base_url, inner.found_feed_url);
                }
                // trace!("CompleteRelativeUrl modified  {} ", inner.found_feed_url);
            }
        }
        StepResult::Continue(Box::new(Notify(inner)))
    }
}

struct Notify(DragInner);
impl Step<DragInner> for Notify {
    fn step(self: Box<Self>) -> StepResult<DragInner> {
        let inner: DragInner = self.0;
        let _r = inner.sourcetree_job_sender.send(SJob::DragUrlEvaluated(
            inner.dragged_url.clone(),
            inner.found_feed_url.clone(),
            inner.error_message.clone(),
            inner.feed_display_title.clone(),
        ));
        if inner.found_feed_url.is_empty() {
            let _r = inner
                .guiproc_job_sender
                .send(Job::AddBottomDisplayErrorMessage(
                    inner.error_message.clone(),
                ));
        }
        StepResult::Stop(inner)
    }
}

// returns the grepped Feed urls
pub fn extract_feed_urls_sloppy(pagetext: &str) -> Vec<String> {
    let mut found_feed_urls: Vec<String> = Vec::default();
    let lines_separated = pagetext.replace('<', "\n<");
    for line in lines_separated.lines() {
        let trimmed = line.trim().to_string();
        if trimmed.len() < 3 {
            continue;
        }
        if !trimmed.contains("<link") {
            continue;
        }
        if !trimmed.contains("rss") {
            continue;
        }
        let parts = trimmed.split(' ');
        let parts_vec = parts
            .into_iter()
            .map(|p| p.to_string())
            .collect::<Vec<String>>();
        let e_first_href = parts_vec.iter().enumerate().find_map(|(n, p)| {
            if p.starts_with("href") {
                return Some(n);
            }
            None
        });

        if let Some(ind) = e_first_href {
            if let Some(assignm) = parts_vec.get(ind) {
                let mut split_r = assignm.split('=');
                let _left = split_r.next();
                let rightpart = split_r
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .join("=");
                // trace!("rightpart:  {:?}", rightpart);
                let probe_url = rightpart.replace(['\"', '>'], "");
                found_feed_urls.push(probe_url);
            }
        }
    }
    found_feed_urls
}
