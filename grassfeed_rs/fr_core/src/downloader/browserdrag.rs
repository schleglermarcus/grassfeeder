use crate::controller::guiprocessor::Job;
use crate::controller::sourcetree::SJob;
use crate::db::errors_repo::ErrorRepo;
use crate::downloader::util::extract_feed_from_website;
use crate::util::Step;
use crate::util::StepResult;
use crate::web::WebFetcherType;
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

        let result = (*inner.web_fetcher).request_url(inner.dragged_url.clone());
        match result.status {
            200 => {
                inner.dragged_url_content = result.content;
            }
            _ => {
                inner.error_message = format!("BrowserEvalStart {}", &result.error_description);
                inner.erro_repo.add_error(
                    -1,
                    result.status as isize,
                    inner.dragged_url.clone(),
                    result.error_description,
                );
                return StepResult::Continue(Box::new(Notify(inner)));
            }
        }
        StepResult::Continue(Box::new(ParseWebpage(inner)))
    }
}

struct ParseWebpage(DragInner);
impl Step<DragInner> for ParseWebpage {
    fn step(self: Box<Self>) -> StepResult<DragInner> {
        let mut inner: DragInner = self.0;
        // let mut feed_url = String::default();
        // let mut extr_err = String::default();
        // let mut parse_err = String::default();
        let extr_r = extract_feed_from_website(&inner.dragged_url_content, &inner.dragged_url);

        if extr_r.is_err() {
            inner.error_message = extr_r.err().unwrap();
            return StepResult::Continue(Box::new(CheckContentIsFeed(inner)));
        }
        inner.found_feed_url = extr_r.unwrap();
        debug!("ParseWebpage OK {:?}", &inner.found_feed_url);
        StepResult::Continue(Box::new(Notify(inner)))
    }
}

struct CheckContentIsFeed(DragInner);
impl Step<DragInner> for CheckContentIsFeed {
    fn step(self: Box<Self>) -> StepResult<DragInner> {
        let mut inner: DragInner = self.0;

        let parse_r = feed_rs::parser::parse(inner.dragged_url_content.as_bytes());
        if parse_r.is_err() {
            inner.error_message += &parse_r.err().unwrap().to_string();
            return StepResult::Continue(Box::new(CheckContentIsFeed(inner)));
        }
        inner.found_feed_url = inner.dragged_url.clone();
        debug!("CheckContentIsFeed OK {:?}", &inner.found_feed_url);
        StepResult::Continue(Box::new(Notify(inner)))
    }
}

struct AnalyzeContentSloppy(DragInner);
impl Step<DragInner> for AnalyzeContentSloppy {
    fn step(self: Box<Self>) -> StepResult<DragInner> {
        let mut inner: DragInner = self.0;

        let extracted: Vec<String> = extract_feed_urls_sloppy(&inner.dragged_url_content);
        if !extracted.is_empty() {
            debug!("FOUND: {:?}", extracted);
            inner.found_feed_url = extracted.first().unwrap().clone();
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
        ));
        StepResult::Stop(inner)
    }
}

// returns the grepped Feed urls
pub fn extract_feed_urls_sloppy(pagetext: &String) -> Vec<String> {
    let mut found_feed_urls: Vec<String> = Vec::default();
    for line in pagetext.lines() {
        let trimmed = line.trim().to_string();
        if !trimmed.contains("<link") {
            continue;
        }
        if !trimmed.contains("rss") {
            continue;
        }
        let parts = trimmed.split(" ");
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
                let mut split_r = assignm.split("=");
                let _left = split_r.next();
                if let Some(r) = split_r.next() {
                    found_feed_urls.push(r.to_string());
                }
            }
        }
    }
    found_feed_urls
}

// #[cfg(test)]mod t_ {    use super::*; }
