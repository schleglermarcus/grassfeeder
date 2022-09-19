use crate::controller::contentlist::match_new_entries_to_existing;
use crate::controller::contentlist::message_from_modelentry;
use crate::controller::contentlist::CJob;
use crate::controller::sourcetree::SJob;
use crate::db::icon_repo::IconRepo;
use crate::db::message::compress;
use crate::db::message::MessageRow;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::util::timestamp_from_utc;
use crate::util::timestamp_now;
use crate::util::Step;
use crate::util::StepResult;
use crate::web::WebFetcherType;
use chrono::DateTime;
use chrono::Local;
use feed_rs::parser::ParseFeedError;
use flume::Sender;
use regex::Regex;

#[derive(Debug)]
pub struct FetchStart(pub FetchInner);

impl FetchStart {
    pub fn new(i: FetchInner) -> Self {
        FetchStart(i)
    }
}

impl Step<FetchInner> for FetchStart {
    fn step(self: Box<Self>) -> StepResult<FetchInner> {
        let inner = &self.0;
        let _r = inner
            .sourcetree_job_sender
            .send(SJob::SetFetchInProgress(inner.fs_repo_id));
        StepResult::Continue(Box::new(DownloadStart(self.0)))
    }
}

struct DownloadStart(FetchInner);
impl Step<FetchInner> for DownloadStart {
    fn step(self: Box<Self>) -> StepResult<FetchInner> {
        let mut inner = self.0;
        let r = (*inner.web_fetcher).request_url(inner.url.clone());
        match r.status {
            200 => {
                inner.download_text = r.content;
                StepResult::Continue(Box::new(EvalStringAndFilter(inner)))
            }
            _ => {
                inner.download_error_happened = true;
                debug!("Http Request {} failed: {}  ", &inner.url, r.status);
                StepResult::Continue(Box::new(NotifyDlStop(inner)))
            }
        }
    }
}

struct EvalStringAndFilter(FetchInner);
impl Step<FetchInner> for EvalStringAndFilter {
    fn step(self: Box<Self>) -> StepResult<FetchInner> {
        let mut inner = self.0;
        let (mut new_list, ts_created, err_text): (Vec<MessageRow>, i64, String) =
            feed_text_to_entries(
                inner.download_text.clone(),
                inner.fs_repo_id,
                inner.url.clone(),
            );
        if !err_text.is_empty() {
            debug!("{:?}", err_text);
            inner.download_error_happened = true;
        }
        let o_err_msg = strange_datetime_recover(&mut new_list, &inner.download_text);
        if let Some(err_msg) = o_err_msg {
            warn!("{} {}", err_msg, &inner.url); // later put this into  error database
        }
        inner.timestamp_created = ts_created;
        let existing_entries = inner.messgesrepo.get_by_src_id(inner.fs_repo_id, false );
        let filtered_list =
            match_new_entries_to_existing(&new_list, &existing_entries, inner.cjob_sender.clone());
        //	filtered_list.iter().for_each(|f|  debug!("F:{} P:{} title={:#?}", f.message_id, f.post_id, f.title) );
        match inner.messgesrepo.insert_tx(&filtered_list) {
            Ok(_num) => {
                inner.download_text.clear();
                StepResult::Continue(Box::new(SetSourceUpdatedExt(inner)))
            }
            Err(e) => {
                error!("storing filtered content entries: {:?}", e);
                StepResult::Continue(Box::new(NotifyDlStop(inner)))
            }
        }
    }
}

struct SetSourceUpdatedExt(FetchInner);
impl Step<FetchInner> for SetSourceUpdatedExt {
    fn step(self: Box<Self>) -> StepResult<FetchInner> {
        let inner = self.0;
        let now = timestamp_now();
        // trace!(            "DL: updating timestamps DB:{}   int:{} ext:{}",            inner.fs_repo_id, now, inner.timestamp_created        );
        inner.subscriptionrepo.update_timestamps(
            inner.fs_repo_id,
            now,
            Some(inner.timestamp_created),
        );
        StepResult::Continue(Box::new(NotifyDlStop(inner)))
    }
}

struct NotifyDlStop(FetchInner);
impl Step<FetchInner> for NotifyDlStop {
    fn step(self: Box<Self>) -> StepResult<FetchInner> {
        let inner = &self.0;
        let _r = inner
            .sourcetree_job_sender
            .send(SJob::StoreFeedCreateUpdate(
                inner.fs_repo_id,
                timestamp_now(),
                inner.timestamp_created,
            ));
        let _r = inner.sourcetree_job_sender.send(SJob::SetFetchFinished(
            inner.fs_repo_id,
            inner.download_error_happened,
        ));

        StepResult::Continue(Box::new(Final(self.0)))
    }
}

struct Final(FetchInner);
impl Step<FetchInner> for Final {
    fn step(self: Box<Self>) -> StepResult<FetchInner> {
        StepResult::Stop(self.0)
    }
}

pub struct FetchInner {
    pub fs_repo_id: isize,
    pub url: String,
    pub cjob_sender: Sender<CJob>,
    pub subscriptionrepo: SubscriptionRepo,
    pub iconrepo: IconRepo,
    pub web_fetcher: WebFetcherType,
    pub download_text: String,
    pub download_error_happened: bool,
    pub download_error_text: String,
    pub sourcetree_job_sender: Sender<SJob>,
    pub timestamp_created: i64,
    pub messgesrepo: MessagesRepo,
}

impl std::fmt::Debug for FetchInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("")
            .field("repo_id", &self.fs_repo_id)
            .field("url", &self.url)
            .field("T", &self.download_text)
            .field("E", &self.download_error_happened)
            .finish()
    }
}

impl PartialEq for FetchInner {
    fn eq(&self, other: &Self) -> bool {
        self.fs_repo_id == other.fs_repo_id
    }
}

/// returns  list of content entries,   timestamp of creation, error_text
/// titles are compressed
pub fn feed_text_to_entries(
    text: String,
    source_repo_id: isize,
    url: String,
) -> (Vec<MessageRow>, i64, String) {
    let mut fce_list: Vec<MessageRow> = Vec::new();
    let mut created_ts: i64 = 0;
    let mut err_text = String::default();
    match feed_rs::parser::parse(text.as_bytes()) {
        Ok(feed) => {
            for e in feed.entries {
                let mut fce = message_from_modelentry(&e);
                fce.subscription_id = source_repo_id;
                fce.title = compress(&fce.title);
                fce.content_text = compress(&fce.content_text);
                fce.categories = compress(&fce.categories);
                fce.author = compress(&fce.author);
                fce_list.push(fce);
            }
            if let Some(utc_date) = feed.updated {
                created_ts = timestamp_from_utc(utc_date);
            }
        }
        Err(e) => {
            let detail = match e {
                ParseFeedError::ParseError(ref kind) => format!("ParseError {:?}", kind),
                ParseFeedError::IoError(ref ioe) => format!("IoError {:?}", ioe),
                ParseFeedError::JsonSerde(ref serde_e) => format!("JsonSerde {:?}", serde_e),
                ParseFeedError::JsonUnsupportedVersion(ref s) => {
                    format!("JsonUnsupportedVersion {:?}", s)
                }
                ParseFeedError::XmlReader(ref xml_e) => {
                    format!("XmlReader {:?}  ", xml_e)
                }
            };
            err_text = format!("Parsing: {}  length={}   {}", &url, text.len(), detail);
        }
    };
    (fce_list, created_ts, err_text)
}

//  modifies the message list, if a date entry can be interpreted
pub fn strange_datetime_recover(
    newmessages: &mut Vec<MessageRow>,
    dl_text: &str,
) -> Option<String> {
    if newmessages.is_empty() {
        return None;
    }

    let invalidpubdatecount = newmessages
        .iter()
        .filter(|m| m.entry_invalid_pubdate)
        .count();
    let mut date_strings: Vec<String> = Vec::default();
    if invalidpubdatecount > 0 {
        let pubdatelines: Vec<String> = dl_text
            .lines()
            .filter(|l| l.contains("<pubDate>"))
            .map(|s| s.trim().to_string())
            .collect::<Vec<String>>();
        let datestrings = pubdatelines
            .iter()
            .filter_map(|s| s.strip_prefix("<pubDate>"))
            .filter_map(|s| s.strip_suffix("</pubDate>"))
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        date_strings = datestrings;
    }
    let mut o_error: Option<String> = None;
    newmessages.iter_mut().enumerate().for_each(|(n, m)| {
        if m.entry_invalid_pubdate && n < date_strings.len() {
            let regex = Regex::new(r":(\d) ").unwrap();
            let date_replaced = regex.replace(&date_strings[n], ":0$1 ");
            match DateTime::parse_from_rfc2822(&date_replaced) {
                Ok(parse_dt) => {
                    let corrected_ts = DateTime::<Local>::from(parse_dt).timestamp();
                    m.entry_src_date = corrected_ts;
                    m.entry_invalid_pubdate = false;
                }
                Err(e) => {
                    o_error = Some(format!(
                        "Error parse_from_rfc2822(`{}`) {:?} ",
                        &date_replaced, &e
                    ));
                }
            }
        }
    });
    o_error
}

// ---

#[cfg(test)]
mod t_ {
    use super::*;
    use crate::db::message::MessageRow;

    //RUST_BACKTRACE=1 cargo watch -s "cargo test  downloader::messages::t_::t_strange_datetime_recover    --lib -- --exact --nocapture "
    #[test]
    fn t_strange_datetime_recover() {
        let mtext = std::fs::read_to_string("tests/data/naturalnews_rss.xml").unwrap();
        let (mut new_list, _ts_created, _err): (Vec<MessageRow>, i64, String) =
            feed_text_to_entries(mtext.clone(), 5, "some-url".to_string());
        println!("prev:   {:?}", new_list[0].entry_src_date);
        let o_msg = strange_datetime_recover(&mut new_list, &mtext);
        assert!(o_msg.is_none());
        assert_eq!(new_list[0].entry_src_date, 1655935140)
    }

    //RUST_BACKTRACE=1 cargo watch -s "cargo test  downloader::messages::t_::dl_entries_breakingnews    --lib -- --exact --nocapture "
    #[ignore]
    #[test]
    fn dl_entries_breakingnews() {
        let filenames = [
            "tests/data/gui_proc_v2.rss",
            "tests/data/breakingnewsworld-2.xml",
        ];
        for filename in filenames {
            println!("FILE={}", filename);
            let (new_list, _ts_created, _err): (Vec<MessageRow>, i64, String) =
                feed_text_to_entries(
                    std::fs::read_to_string(filename).unwrap(),
                    5,
                    "some-url".to_string(),
                );
            assert!(new_list.get(0).unwrap().entry_src_date > 0);
        }
    }

    //RUST_BACKTRACE=1 cargo watch -s "cargo test   downloader::messages::t_::feed_text_to_entries_naturalnews  --lib -- --exact --nocapture   "
    #[test]
    fn feed_text_to_entries_naturalnews() {
        let filename = "tests/data/naturalnews_rss.xml";
        let contents = std::fs::read_to_string(filename).unwrap();
        let (new_list, _ts_created, _err): (Vec<MessageRow>, i64, String) =
            feed_text_to_entries(contents.clone(), 5, "some-url".to_string());
        assert_eq!(new_list.len(), 30);
        assert_eq!(new_list[1].entry_src_date, 1655877600);
        assert_eq!(new_list[2].entry_src_date, 1655877600);
    }

    // RUST_BACKTRACE=1 cargo watch -s "cargo test  downloader::messages::t_:feed_text_to_entries_xkcd  --lib -- --exact --nocapture "
    #[test]
    fn feed_text_to_entries_xkcd() {
        let filename = "tests/data/xkcd_atom.xml";
        let contents = std::fs::read_to_string(filename).unwrap();
        let (new_list, _ts_created, _err): (Vec<MessageRow>, i64, String) =
            feed_text_to_entries(contents.clone(), 5, "some-url".to_string());
        assert_eq!(new_list.len(), 4);
    }

    //RUST_BACKTRACE=1 cargo watch -s "cargo test   downloader::messages::t_:feed_text_to_entries_tages  --lib -- --exact --nocapture "
    // A date entry is not contained here
    #[test]
    fn feed_text_to_entries_tages() {
        let filename = "tests/data/tagesschau.rdf";
        let contents = std::fs::read_to_string(filename).unwrap();
        let (new_list, _ts_created, _err): (Vec<MessageRow>, i64, String) =
            feed_text_to_entries(contents.clone(), 5, "some-url".to_string());
        assert_eq!(new_list.len(), 46);
        assert_eq!(
            new_list.get(0).unwrap().post_id,
            "https://www.tagesschau.de/inland/regierungserklaerung-scholz-gipfeltreffen-103.html"
        );
    }

    //RUST_BACKTRACE=1 cargo watch -s "cargo test  downloader::messages::t_:feed_text_to_entries_local  --lib -- --exact --nocapture "
    #[test]
    fn feed_text_to_entries_local() {
        let filename = "tests/data/gui_proc_rss2_v1.rss";
        let contents = std::fs::read_to_string(filename).unwrap();
        let (new_list, ts_created, _err): (Vec<MessageRow>, i64, String) =
            feed_text_to_entries(contents.clone(), 5, "some-url".to_string());
        assert_eq!(new_list.len(), 2);
        assert_eq!(ts_created, 1636573888);
    }
}
