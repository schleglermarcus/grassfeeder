use crate::controller::contentlist::match_new_entries_to_existing;
use crate::controller::contentlist::CJob;
use crate::controller::sourcetree::SJob;
use crate::db::errors_repo::ErrorRepo;
use crate::db::icon_repo::IconRepo;
use crate::db::message::compress;
use crate::db::message::MessageRow;
use crate::db::messages_repo::IMessagesRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::downloader::util::workaround_https_declaration;
use crate::util::remove_invalid_chars_from_input;
use crate::util::timestamp_from_utc;
use crate::util::timestamp_now;
use crate::util::Step;
use crate::util::StepResult;
use crate::web::WebFetcherType;
use chrono::DateTime;
use chrono::Local;
use feed_rs::model::Entry;
use feed_rs::parser::ParseFeedError;
use flume::Sender;
use regex::Regex;
use url::Url;

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
    pub erro_repo: ErrorRepo,
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
                inner.erro_repo.add_error(
                    inner.fs_repo_id,
                    r.status as isize,
                    inner.url.to_string(),
                    r.error_description,
                );
                StepResult::Continue(Box::new(NotifyDlStop(inner)))
            }
        }
    }
}

struct EvalStringAndFilter(FetchInner);
impl Step<FetchInner> for EvalStringAndFilter {
    fn step(self: Box<Self>) -> StepResult<FetchInner> {
        let mut inner = self.0;
        let dl_text = workaround_https_declaration(inner.download_text.clone());
        let (mut new_list, ts_created, err_text): (Vec<MessageRow>, i64, String) =
            feed_text_to_entries(dl_text, inner.fs_repo_id, inner.url.clone());
        if !err_text.is_empty() {
            inner
                .erro_repo
                .add_error(inner.fs_repo_id, 0, inner.url.to_string(), err_text);
        }
        let o_err_msg = strange_datetime_recover(&mut new_list, &inner.download_text);
        if let Some(err_msg) = o_err_msg {
            inner
                .erro_repo
                .add_error(inner.fs_repo_id, 0, inner.url.to_string(), err_msg);
        }
        inner.timestamp_created = ts_created;
        let existing_entries = inner.messgesrepo.get_by_src_id(inner.fs_repo_id, false);
        let filtered_list =
            match_new_entries_to_existing(&new_list, &existing_entries, inner.cjob_sender.clone());
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
                let (mut fce, err_t) = message_from_modelentry(&e);
                fce.subscription_id = source_repo_id;
                fce.title = compress(&fce.title);
                fce.content_text = compress(&fce.content_text);
                fce.categories = compress(&fce.categories);
                fce.author = compress(&fce.author);
                fce_list.push(fce);
                err_text.push_str(&err_t);
            }
            if let Some(utc_date) = feed.updated {
                created_ts = timestamp_from_utc(utc_date);
            }
        }
        Err(e) => {
            let detail = match e {
                ParseFeedError::ParseError(ref kind) => format!("ParseError {kind:?}"),
                ParseFeedError::IoError(ref ioe) => format!("IoError {ioe:?}"),
                ParseFeedError::JsonSerde(ref serde_e) => format!("JsonSerde {serde_e:?}"),
                ParseFeedError::JsonUnsupportedVersion(ref s) => {
                    format!("JsonUnsupportedVersion {s:?}")
                }
                ParseFeedError::XmlReader(ref xml_e) => {
                    format!("XmlReader {xml_e:?}  ")
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

///
///  takes the last of media[]  and brings it into enclosure_url
///
///   filter_by_iso8859_1().0;    // also removes umlauts
///  https://docs.rs/feed-rs/latest/feed_rs/model/struct.Entry.html#structfield.published
///         * RSS 2 (optional) "pubDate": Indicates when the item was published.
///
///  if title  contains invalid chars (for instance  & ), the Option<title>  is empty
/// returns  converted Message-Entry,  Error-Text
pub fn message_from_modelentry(me: &Entry) -> (MessageRow, String) {
    let mut msg = MessageRow::default();
    let mut published_ts: i64 = 0;
    let mut error_text = String::default();
    if let Some(publis) = me.published {
        published_ts = DateTime::<Local>::from(publis).timestamp();
    } else {
        if let Some(upd) = me.updated {
            published_ts = DateTime::<Local>::from(upd).timestamp();
        }
        msg.entry_invalid_pubdate = true;
    }
    msg.entry_src_date = published_ts;
    msg.fetch_date = crate::util::timestamp_now();
    msg.message_id = -1;
    let linklist = me
        .links
        .iter()
        // .inspect(|ml| debug!("ML: {:?} {:?}", ml.rel, ml.href))
        .filter(|ml| {
            if let Some(typ) = &ml.media_type {
                if typ.contains("xml") {
                    return false;
                }
            }
            true
        })
        .filter(|ml| {
            if let Some(rel) = &ml.rel {
                if rel.contains("replies") {
                    return false;
                }
            }
            true
        })
        .collect::<Vec<&feed_rs::model::Link>>();
    if let Some(link_) = linklist.first() {
        msg.link = link_.href.clone();
    }
    if let Some(summary) = me.summary.clone() {
        if !summary.content.is_empty() {
            msg.content_text = summary.content;
        }
    }
    msg.post_id = me.id.clone();
    if let Some(c) = me.content.clone() {
        if let Some(b) = c.body {
            msg.content_text = b
        }
        if let Some(enc) = c.src {
            msg.enclosure_url = enc.href
        }
    }
    for media in &me.media {
        for cont in &media.content {
            if let Some(m_url) = &cont.url {
                let u: Url = m_url.clone();
                if u.domain().is_some() {
                    msg.enclosure_url =
                        format!("{}://{}{}", u.scheme(), u.domain().unwrap(), u.path());
                }
            }
        }
        if msg.content_text.is_empty() {
            if let Some(descrip) = &media.description {
                if descrip.content_type.to_string().starts_with("text") {
                    msg.content_text = descrip.content.clone();
                }
            }
        }
    }

    if let Some(t) = me.title.clone() {
        let mut filtered = remove_invalid_chars_from_input(t.content);
        filtered = filtered.trim().to_string();
        msg.title = filtered;
    } else {
        error_text = format!("Message ID {} has no valid title.", &me.id);
        msg.title = msg.post_id.clone();
    }
    let authorlist = me
        .authors
        .iter()
        .map(|author| author.name.clone())
        .filter(|a| a.as_str() != "author")
        .map(remove_invalid_chars_from_input)
        .collect::<Vec<String>>()
        .join(", ");
    let cate_list = me
        .categories
        .iter()
        .map(|cat| cat.term.clone())
        .map(remove_invalid_chars_from_input)
        .collect::<Vec<String>>()
        .join(", ");
    msg.author = authorlist;
    msg.categories = cate_list;
    (msg, error_text)
}

// ---

#[cfg(test)]
mod t_ {
    use super::*;

    use crate::db::message::MessageRow;
    use crate::util::db_time_to_display_nonnull;
    use feed_rs::parser;

    // #[ignore]
    #[test]
    fn parse_convert_entry_content_simple() {
        let rss_str = r#" <?xml version="1.0" encoding="UTF-8"?>
	 	        <rss   version="2.0"  xmlns:content="http://purl.org/rss/1.0/modules/content/" >
	 	        <channel>
	 	         <item>
	 	            <title>Rama Dama</title>
	 	              <description>Bereits sein Regie-Erstling war ein Hit</description>
	 	              <content:encoded>Lorem1</content:encoded>
	 	         </item>
	 	        </channel>
	 	        </rss>"#;
        let feeds = parser::parse(rss_str.as_bytes()).unwrap();
        let first_entry = feeds.entries.get(0).unwrap();
        let fce: MessageRow = message_from_modelentry(&first_entry).0;
        assert_eq!(fce.content_text, "Lorem1");
    }

    /*
    id: String

    A unique identifier for this item with a feed. If not supplied it is initialised to a hash of the first link or a UUID if not available.

        Atom (required): Identifies the entry using a universally unique and permanent URI.
        RSS 2 (optional) “guid”: A string that uniquely identifies the item.
        RSS 1: does not specify a unique ID as a separate item, but does suggest the URI should be “the same as the link” so we use a hash of the link if found
        JSON Feed: is unique for that item for that feed over time.

    */
    // #[ignore]
    #[test]
    fn parse_feed_with_namespaces() {
        let rss_str = r#" <?xml version="1.0" encoding="UTF-8"?>
	 	        <rss xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:content="http://purl.org/rss/1.0/modules/content/" version="2.0">
	 	        <channel>
	 	            <title>Neu im Kino</title>
	 	            <item>
	 	              <title>Rama Dama</title>
	 	              <dc:creator>Kino.de Redaktion</dc:creator>
	 	              <content:encoded>Lorem2</content:encoded>
	 				  <guid>1234</guid>
	 	            </item>
	 	        </channel>
	 	        </rss>"#;
        let feeds = parser::parse(rss_str.as_bytes()).unwrap();
        let first_entry = feeds.entries.get(0).unwrap();
        assert!(!first_entry.authors.is_empty());
        assert_eq!(first_entry.authors[0].name, "Kino.de Redaktion");
        let fce: MessageRow = message_from_modelentry(&first_entry).0;
        assert_eq!(fce.content_text, "Lorem2");
        assert_eq!(fce.post_id, "1234");
    }

    // #[ignore]
    #[test]
    fn message_from_modelentry_3() {
        let rsstext = r#" <?xml version="1.0" encoding="UTF-8"?>
	 	<rss xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:content="http://purl.org/rss/1.0/modules/content/" version="2.0">
	 	  <channel>
	 	    <description>Alle neuen Filme in den deutschen Kinos</description>
	 	    <language>de</language>
	 	    <copyright>Copyright 2021 Kino.de</copyright>
	 	    <title>Neu im Kino</title>
	 	    <lastBuildDate>Wed, 10 Nov 2021 00:12:03 +0100</lastBuildDate>
	 	    <link>https://www.kino.de/rss/stars</link>
	 	    <item>
	 	      <dc:creator>Kino.de Redaktion</dc:creator>
	 	      <description>Bereits sein Regie-Erstling war ein Hit</description>
	 	      <content:encoded>Felix Zeiler verbringt</content:encoded>
	 	      <enclosure url="https://static.kino.de/rama-dama-1990-film-rcm1200x0u.jpg" type="image/jpeg" length="153553"/>
	 	      <pubDate>Wed, 13 Oct 2021 12:00:00 +0200</pubDate>
	 	      <title>Rama Dama</title>
	 	      <link>https://www.kino.de/film/rama-dama-1990/</link>
	 	      <guid isPermaLink="true">https://www.kino.de/film/rama-dama-1990/</guid>
	 	    </item>
	 	  </channel>
	 	</rss>"#;
        let feeds = parser::parse(rsstext.as_bytes()).unwrap();
        let first_entry = feeds.entries.get(0).unwrap();
        let fce: MessageRow = message_from_modelentry(&first_entry).0;
        assert_eq!(fce.content_text, "Felix Zeiler verbringt");
        assert_eq!(
            fce.enclosure_url,
            "https://static.kino.de/rama-dama-1990-film-rcm1200x0u.jpg"
        );
    }

    #[test]
    fn message_from_modelentry_4() {
        let rsstext = r#" <?xml version="1.0" encoding="UTF-8"?>
	 		<?xml-stylesheet type="text/xsl" media="screen" href="/~d/styles/rss2enclosuresfull.xsl"?>
	 		<?xml-stylesheet type="text/css" media="screen" href="http://feeds.feedburner.com/~d/styles/itemcontent.css"?>
	 		<rss xmlns:media="http://search.yahoo.com/mrss/" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd" xmlns:feedburner="http://rssnamespace.org/feedburner/ext/1.0" version="2.0">
	 		  <channel>
	 		    <title>THE FINANCIAL ARMAGEDDON BLOG</title>
	 		    <link>http://financearmageddon.blogspot.com/</link>
	 		    <description>&lt;i&gt;&lt;small&gt;THE | ECONOMIC COLLAPSE  | FINANCIAL ARMAGEDDON |  MELTDOWN | BLOG  is digging for the Truth Deep Down the Rabbit Hole , in Order to Prepare to Survive &amp;amp; Thrive the coming &lt;b&gt;Financial Apocalypse&lt;/b&gt; &amp;amp; &lt;b&gt;Economic Collapse&lt;/b&gt; &amp;amp;  be Ready for The Resistance to Tyranny and The NWO ,  Minds are like parachutes.......They only function when they are Open so Free Your Mind and come on join the ride&lt;/small&gt;&lt;/i&gt;</description>
	 		    <language>en</language>
	 		    <lastBuildDate>Wed, 10 Nov 2021 14:51:28 PST</lastBuildDate>
	 		<item>
	 	      <title>Warning : A 2 Quadrillions Debt Bubble by 2030     https://youtu.be/x6lmb992L0Q</title>
	 	      <link>http://feedproxy.google.com/~r/blogspot/cwWR/~3/wFtNHz9TStU/warning-2-quadrillions-debt-bubble-by.html</link>
	 	      <author>noreply@blogger.com (Politico Cafe)</author>
	 	      <pubDate>Mon, 01 Nov 2021 07:50:19 PDT</pubDate>
	 	      <guid isPermaLink="false">tag:blogger.com,1999:blog-8964382413486690048.post-7263323075085527050</guid>
	 	      <media:thumbnail url="https://img.youtube.com/vi/x6lmb992L0Q/default.jpg" height="72" width="72"/>
	 	      <thr:total xmlns:thr="http://purl.org/syndication/thread/1.0">0</thr:total>
	 	      <description>Warning : A 2 Quadrillions Debt Bubble by 2030     https://youtu.be/x6lmb992L0Q
	 	Central Banks are the new  Feudalism.
	 	All property is being concentrated into a few hands via Fiat and zero interest.
	 	Serfdom is the endgame.
	 	Central bankers were handed the Midas curse half a century...&lt;br/&gt;
	 	&lt;br/&gt;
	 	[[ This is a content summary only. Visit http://FinanceArmageddon.blogspot.com or  http://lindseywilliams101.blogspot.com  for full links, other content, and more! ]]&lt;img src="http://feeds.feedburner.com/~r/blogspot/cwWR/~4/wFtNHz9TStU" height="1" width="1" alt=""/&gt;</description>
	 	      <feedburner:origLink>http://financearmageddon.blogspot.com/2021/11/warning-2-quadrillions-debt-bubble-by.html</feedburner:origLink>
	 	    </item>
	 	  </channel>
	 	</rss>"#;
        let feeds = parser::parse(rsstext.as_bytes()).unwrap();
        let first_entry = feeds.entries.get(0).unwrap();
        let fce: MessageRow = message_from_modelentry(&first_entry).0;
        assert!(fce.content_text.len() > 10);
    }

    // #[allow(dead_code)]
    #[test]
    fn from_modelentry_naturalnews_copy() {
        let rsstext = r#"<?xml version="1.0" encoding="ISO-8859-1"?>
	 <rss xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd" version="2.0">
	   <channel>
	     <title>NaturalNews.com</title>
	     <lastBuildDate>Wed, 22 Jun 2022 00:00:00 CST</lastBuildDate>
	     <item>
	       <title><![CDATA[RED ALERT: Entire U.S. supply of diesel engine oil may be wiped out in 8 weeks&#8230; no more oil until 2023 due to &#8220;Force Majeure&#8221; additive chemical shortages]]></title>
	       <description><![CDATA[<table><tr><td><img src='wp-content/uploads/sites/91/2022/06/HRR-2022-06-22-Situation-Update_thumbnail.jpg' width='140' height='76' /></td><td valign='top'>(NaturalNews) <p> (Natural News)&#10; As if we all needed something else to add to our worries, a potentially catastrophic situation is emerging that threatens to wipe out the entire supply of diesel engine oil across the United States, leaving the country with no diesel engine oil until 2023.This isn't merely a rumor: We've confirmed this is &#x02026; [Read More...]</p></td></tr></table>]]></description>
	       <author><![CDATA[Mike Adams]]></author>
	       <pubDate>Wed, 22 Jun 2022  15:59:0 CST</pubDate>
	       <link><![CDATA[https://www.naturalnews.com/2022-06-22-red-alert-entire-us-supply-of-diesel-engine-oil-wiped-out.html]]></link>
	       <guid><![CDATA[https://www.naturalnews.com/2022-06-22-red-alert-entire-us-supply-of-diesel-engine-oil-wiped-out.html]]></guid>
	     </item>
	   </channel>
	 </rss>     "#;

        let feeds = parser::parse(rsstext.as_bytes()).unwrap();
        let first_entry = feeds.entries.get(0).unwrap();
        let fce: MessageRow = message_from_modelentry(&first_entry).0;
        println!(
            "entry_src_date={:?}   ",
            db_time_to_display_nonnull(fce.entry_src_date),
        );
        assert!(fce.content_text.len() > 10);
    }
}
