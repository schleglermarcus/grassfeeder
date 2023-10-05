use crate::config::configmanager::ConfigManager;
use crate::controller::contentlist::CJob;
use crate::controller::contentlist::FeedContents;
use crate::controller::contentlist::IContentList;
use crate::controller::guiprocessor::GuiProcessor;
use crate::controller::guiprocessor::Job;
use crate::controller::isourcetree::ISourceTreeController;
use crate::controller::sourcetree::SJob;
use crate::controller::sourcetree::SourceTreeController;
use crate::controller::timer::Timer;
use crate::db::errors_repo::ErrorRepo;
use crate::db::icon_repo::IconRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::downloader::browserdrag::BrowserEvalStart;
use crate::downloader::browserdrag::DragInner;
use crate::downloader::comprehensive::ComprStart;
use crate::downloader::comprehensive::ComprehensiveInner;
use crate::downloader::db_clean::CleanerInner;
use crate::downloader::db_clean::CleanerStart;
use crate::downloader::icons::IconInner;
use crate::downloader::icons::IconLoadStart;
use crate::downloader::launch_web::LaunchInner;
use crate::downloader::launch_web::LaunchWebBrowserStart;
use crate::downloader::messages::FetchInner;
use crate::downloader::messages::FetchStart;
use crate::util::StepResult;
use crate::web::httpfetcher::HttpFetcher;
use crate::web::WebFetcherType;
use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use context::TimerEvent;
use context::TimerReceiver;
use context::TimerRegistry;
use flume::Sender;
use resources::parameter::DOWNLOADER_MAX_NUM_THREADS;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;

pub static KEEPRUNNING: AtomicBool = AtomicBool::new(true);
pub const CONF_DOWNLOADER_THREADS: &str = "DownloaderThreads";
pub const CONF_DATABASES_CLEANUP: &str = "DatabasesCleanup";
pub const DOWNLOADER_THREADS_DEFAULT: u8 = 2;
pub const DOWNLOADER_LOOP_DELAY_S: u8 = 1;
pub const DOWNLOADER_LOOP_WAIT_MS: u64 = 200; // between downloader queue requests
pub const DOWNLOADER_JOB_QUEUE: usize = 2000;
pub const DLKIND_MAX: usize = 7;

pub trait IDownloader {
    fn shutdown(&mut self);
    fn is_running(&self) -> bool;
    fn get_config(&self) -> Config;
    fn set_conf_num_threads(&mut self, n: u8);
    fn get_kind_list(&self) -> Vec<u8>;
    fn add_update_source(&self, f_source_repo_id: isize);
    fn new_feedsource_request(&self, fs_edit_url: &str);
    fn load_icon(&self, fs_id: isize, fs_url: String, old_icon_id: usize);
    fn cleanup_db(&self);
    fn get_queue_size(&self) -> usize;
    fn browser_drag_request(&self, dragged_url: &str);
    fn launch_webbrowser(&self, url: String, cl_id: isize, list_pos: u32);
    fn get_statistics(&self) -> [u32; DLKIND_MAX];
}

#[derive(Debug, PartialEq)]
pub enum DLJob {
    None, // to trigger the loop only
    Feed(FetchInner),
    Icon(IconInner),
    ComprehensiveFeed(ComprehensiveInner),
    CleanDatabase(CleanerInner),
    BrowserDragEvaluation(DragInner),
    LaunchWebBrowser(LaunchInner),
}

pub trait DLKind {
    fn kind(&self) -> u8;
    fn hostname(&self) -> Option<String> {
        None
    }
    fn subscription_id(&self) -> isize;
}

impl DLKind for DLJob {
    /// see  [crate::guiprocessor::dl_char_for_kind]
    fn kind(&self) -> u8 {
        match self {
            DLJob::None => 0,
            DLJob::Feed(_) => 1,
            DLJob::Icon(_) => 2,
            DLJob::ComprehensiveFeed(_) => 3,
            DLJob::CleanDatabase(_) => 4,
            DLJob::BrowserDragEvaluation(_) => 5,
            DLJob::LaunchWebBrowser(_) => 6,
        }
    }

    fn hostname(&self) -> Option<String> {
        match self {
            DLJob::Feed(fetch_inner) => Downloader::host_from_url(&fetch_inner.url),
            DLJob::Icon(icon_inner) => Downloader::host_from_url(&icon_inner.feed_url),
            _ => None,
        }
    }

    fn subscription_id(&self) -> isize {
        match self {
            DLJob::None => -1,
            DLJob::Feed(inner) => inner.fs_repo_id,
            DLJob::Icon(inner) => inner.subs_id,
            DLJob::ComprehensiveFeed(_) => -2,
            DLJob::CleanDatabase(_) => -3,
            DLJob::BrowserDragEvaluation(_) => -4,
            DLJob::LaunchWebBrowser(_) => -5,
        }
    }
}

pub struct Downloader {
    joinhandles: Vec<thread::JoinHandle<()>>,
    subscriptionrepo_r: Rc<RefCell<dyn ISubscriptionRepo>>,
    iconrepo_r: Rc<RefCell<IconRepo>>,
    web_fetcher: WebFetcherType,
    pub contentlist_job_sender: Option<Sender<CJob>>,
    pub source_c_sender: Option<Sender<SJob>>,
    pub gp_job_sender: Option<Sender<Job>>,
    configmanager_r: Rc<RefCell<ConfigManager>>,
    config: Config,
    pub busy_indicators: Arc<RwLock<[(u8, String); DOWNLOADER_MAX_NUM_THREADS]>>,
    messagesrepo: Rc<RefCell<MessagesRepo>>,
    job_queue: Arc<RwLock<VecDeque<DLJob>>>,
    erro_repo: Rc<RefCell<ErrorRepo>>,
    call_statistic: RefCell<[u32; DLKIND_MAX]>,
}

impl Downloader {
    pub fn new(
        fetcher: WebFetcherType,
        subscr_r: Rc<RefCell<dyn ISubscriptionRepo>>,
        icon_repo_r: Rc<RefCell<IconRepo>>,
        cm_r: Rc<RefCell<ConfigManager>>,
        msgrepo: Rc<RefCell<MessagesRepo>>,
        err_repo: Rc<RefCell<ErrorRepo>>,
    ) -> Self {
        Downloader {
            joinhandles: Vec::default(),
            subscriptionrepo_r: subscr_r,
            iconrepo_r: icon_repo_r,
            web_fetcher: fetcher,
            contentlist_job_sender: None, // cjob_sender,
            source_c_sender: None,        // sjob_sender,
            gp_job_sender: None,          // guiprocessor jobs
            configmanager_r: cm_r,
            config: Config::default(),
            busy_indicators: Arc::new(RwLock::new(Default::default())),
            messagesrepo: msgrepo,
            job_queue: Arc::new(RwLock::new(VecDeque::default())),
            erro_repo: err_repo,
            call_statistic: RefCell::new([0; DLKIND_MAX]),
        }
    }

    pub fn new_ac(ac: &AppContext) -> Self {
        let fetcher: WebFetcherType = Arc::new(Box::new(HttpFetcher {}));
        let subscr_r: Rc<RefCell<dyn ISubscriptionRepo>> =
            (*ac).get_rc::<SubscriptionRepo>().unwrap();
        let iconrepo_r: Rc<RefCell<IconRepo>> = (*ac).get_rc::<IconRepo>().unwrap();
        let cm_r = (*ac).get_rc::<ConfigManager>().unwrap();
        let msgrepo = (*ac).get_rc::<MessagesRepo>().unwrap();
        let errors_r = (*ac).get_rc::<ErrorRepo>().unwrap();
        Downloader::new(fetcher, subscr_r, iconrepo_r, cm_r, msgrepo, errors_r)
    }

    pub fn startup(&mut self) {
        if self.config.num_downloader_threads == 0
            || self.config.num_downloader_threads > DOWNLOADER_MAX_NUM_THREADS as u8
        {
            warn!(
                "Invalid Number of downloader threads: {}",
                self.config.num_downloader_threads
            );
            self.config.num_downloader_threads = DOWNLOADER_THREADS_DEFAULT;
        }
        for n in 0..self.config.num_downloader_threads {
            let gp_sender: Sender<Job> = self.gp_job_sender.as_ref().unwrap().clone();
            let queue_a = self.job_queue.clone();
            let busy_a = self.busy_indicators.clone();
            let builder = thread::Builder::new().name(format!("dl_{n}"));
            let h = builder
                .spawn(move || loop {
                    let mut skip_it: bool = false;
                    let mut hostname = String::default();
                    if let Some(ref_job) = (*queue_a).read().unwrap().front() {
                        if let Some(ref hostnam) = ref_job.hostname() {
                            hostname = hostnam.clone();
                            for (_kind, hn) in (*busy_a).read().unwrap().iter() {
                                if !hn.is_empty() && hn.eq(hostnam) {
                                    // trace!("HOST {} in use with {} {} , pushback  ", hn, kind, n);
                                    skip_it = true;
                                }
                            }
                        }
                    }
                    if skip_it {
                        let mut q_w = (*queue_a).write().unwrap();
                        if let Some(dl_job) = q_w.pop_front() {
                            q_w.push_back(dl_job);
                        }
                    } else {
                        let o_job = (*queue_a).write().unwrap().pop_front();
                        if let Some(dljob) = o_job {
                            (*busy_a).write().unwrap()[n as usize] = (dljob.kind(), hostname);
                            Self::process_job(dljob, gp_sender.clone(), n);
                            (*busy_a).write().unwrap()[n as usize] = (0, String::default());
                        }
                    }
                    let k = KEEPRUNNING.load(Ordering::Relaxed);
                    if k {
                        thread::sleep(Duration::from_millis(DOWNLOADER_LOOP_WAIT_MS));
                    } else {
                        break;
                    }
                })
                .unwrap();
            self.joinhandles.push(h);
        }
    }

    fn add_to_queue(&self, dljob: DLJob) {
        self.call_statistic.borrow_mut()[dljob.kind() as usize] += 1;
        if !(*self.job_queue).read().unwrap().contains(&dljob) {
            (*self.job_queue).write().unwrap().push_back(dljob);
        }
    }

    /// returns   used time in milliseconds
    fn process_job(dljob: DLJob, gp_sender: Sender<Job>, proc_num: u8) -> u64 {
        let now = std::time::Instant::now();
        let job_kind = dljob.kind();
        let subs_id = dljob.subscription_id();
        let _r = gp_sender.send(Job::DownloaderJobStarted(proc_num, job_kind));
        let job_description = format!("{}  {:?}", std::thread::current().name().unwrap(), &dljob);
        let job_hostname = dljob.hostname().unwrap_or_default();
        match dljob {
            DLJob::None => {}
            DLJob::Feed(i) => {
                let _i = StepResult::start(Box::new(FetchStart::new(i)));
            }
            DLJob::Icon(i) => {
                let _i = StepResult::start(Box::new(IconLoadStart::new(i)));
            }
            DLJob::ComprehensiveFeed(i) => {
                let _i = StepResult::start(Box::new(ComprStart::new(i)));
            }
            DLJob::CleanDatabase(i) => {
                let _i = StepResult::start(Box::new(CleanerStart::new(i)));
            }
            DLJob::BrowserDragEvaluation(i) => {
                let _i = StepResult::start(Box::new(BrowserEvalStart::new(i)));
            }
            DLJob::LaunchWebBrowser(i) => {
                let _i = StepResult::start(Box::new(LaunchWebBrowserStart::new(i)));
            }
        }
        let elapsedms = now.elapsed().as_millis();
        let _r = gp_sender.send(Job::DownloaderJobFinished(
            subs_id,
            proc_num,
            job_kind,
            elapsedms as u32,
            job_description,
            job_hostname,
        ));
        elapsedms as u64
    }

    pub fn host_from_url(url: &String) -> Option<String> {
        match url::Url::parse(url) {
            Ok(parsed) => {
                if let Some(hoststr) = parsed.host_str() {
                    return Some(hoststr.to_string());
                }
            }
            Err(e) => {
                debug!("host_from_url({}) ERR:{:?}", &url, e);
            }
        }
        None
    }
}

impl IDownloader for Downloader {
    fn get_kind_list(&self) -> Vec<u8> {
        (*self.busy_indicators)
            .read()
            .unwrap()
            .iter()
            .map(|k_u| k_u.0)
            .collect::<Vec<u8>>()
    }

    fn add_update_source(&self, f_source_repo_id: isize) {
        let o_fse = (*self.subscriptionrepo_r)
            .borrow()
            .get_by_index(f_source_repo_id);
        if o_fse.is_none() {
            warn!("cannot get FSE    {}  ", f_source_repo_id);
            return;
        }
        let fse = o_fse.unwrap();
        if fse.is_folder {
            warn!(" fetch_single    {}  but is folder ", f_source_repo_id);
            return;
        }
        let subscription_repo = SubscriptionRepo::by_existing_connection(
            (*self.subscriptionrepo_r).borrow().get_connection(),
        );
        let icon_repo = IconRepo::by_existing_list((*self.iconrepo_r).borrow().get_list());
        let msgrepo = MessagesRepo::new_by_connection(
            (*self.messagesrepo).borrow().get_ctx().get_connection(),
        );
        let errors_rep = ErrorRepo::by_connection((*self.erro_repo).borrow().get_connection());
        let new_fetch_job = FetchInner {
            fs_repo_id: f_source_repo_id,
            url: fse.url,
            cjob_sender: self.contentlist_job_sender.as_ref().unwrap().clone(),
            subscriptionrepo: subscription_repo,
            iconrepo: icon_repo,
            web_fetcher: self.web_fetcher.clone(),
            download_error_happened: false,
            sourcetree_job_sender: self.source_c_sender.as_ref().unwrap().clone(),
            timestamp_created: 0,
            messgesrepo: msgrepo,
            download_text: String::default(),
            download_error_text: String::default(),
            erro_repo: errors_rep,
        };
        self.add_to_queue(DLJob::Feed(new_fetch_job));
    }

    fn load_icon(&self, subsid: isize, feedurl: String, old_icon_id: usize) {
        let icon_repo = IconRepo::by_existing_list((*self.iconrepo_r).borrow().get_list());
        let subscription_repo = SubscriptionRepo::by_existing_connection(
            (*self.subscriptionrepo_r).borrow().get_connection(),
        );
        let errors_rep = ErrorRepo::by_connection((*self.erro_repo).borrow().get_connection());
        let dl_inner = IconInner {
            subs_id: subsid,
            feed_url: feedurl,
            icon_url: String::default(),
            iconrepo: icon_repo,
            web_fetcher: self.web_fetcher.clone(),
            download_error_happened: false,
            icon_bytes: Vec::default(),
            fs_icon_id_old: old_icon_id as isize,
            sourcetree_job_sender: self.source_c_sender.as_ref().unwrap().clone(),
            feed_homepage: String::default(),
            feed_download_text: String::default(),
            subscriptionrepo: subscription_repo,
            erro_repo: errors_rep,
            image_icon_kind: Default::default(),
            compressed_icon: Default::default(),
        };
        self.add_to_queue(DLJob::Icon(dl_inner));
    }

    fn new_feedsource_request(&self, fs_edit_url: &str) {
        let icon_repo = IconRepo::by_existing_list((*self.iconrepo_r).borrow().get_list());
        let inner = ComprehensiveInner {
            feed_url_edit: fs_edit_url.to_string(),
            icon_url: String::default(),
            iconrepo: icon_repo,
            web_fetcher: self.web_fetcher.clone(),
            download_error_happened: false,
            icon_bytes: Vec::default(),
            icon_id: -1,
            sourcetree_job_sender: self.source_c_sender.as_ref().unwrap().clone(),
            feed_homepage: String::default(),
            feed_title: String::default(),
            url_download_text: String::default(),
        };
        self.add_to_queue(DLJob::ComprehensiveFeed(inner));
    }

    fn cleanup_db(&self) {
        let msg_keep_count: i32 = (*self.configmanager_r)
            .borrow()
            .get_val_int(FeedContents::CONF_MSG_KEEP_COUNT)
            .unwrap_or(-1) as i32;
        let subs_repo = SubscriptionRepo::by_existing_connection(
            (*self.subscriptionrepo_r).borrow().get_connection(),
        );
        let msgrepo1 = MessagesRepo::new_by_connection(
            (*self.messagesrepo).borrow().get_ctx().get_connection(),
        );
        let iconrepo = IconRepo::by_existing_list((*self.iconrepo_r).borrow().get_list());
        let errors_rep = ErrorRepo::by_connection((*self.erro_repo).borrow().get_connection());
        let cleaner_i = CleanerInner::new(
            self.contentlist_job_sender.as_ref().unwrap().clone(),
            self.source_c_sender.as_ref().unwrap().clone(),
            subs_repo,
            msgrepo1,
            iconrepo,
            msg_keep_count,
            errors_rep,
        );
        self.add_to_queue(DLJob::CleanDatabase(cleaner_i));
    }

    fn shutdown(&mut self) {
        KEEPRUNNING.store(false, Ordering::Relaxed);
        self.add_to_queue(DLJob::None);
        while !self.joinhandles.is_empty() {
            let h = self.joinhandles.remove(0);
            h.join().unwrap();
        }
    }

    fn is_running(&self) -> bool {
        !self.joinhandles.is_empty()
    }

    fn get_config(&self) -> Config {
        self.config.clone()
    }

    fn set_conf_num_threads(&mut self, n: u8) {
        if n < 1 || n > DOWNLOADER_MAX_NUM_THREADS as u8 {
            error!("conf_num_threads wrong {}", n);
            return;
        }
        self.config.num_downloader_threads = n;
        (*self.configmanager_r)
            .borrow()
            .set_val(CONF_DOWNLOADER_THREADS, n.to_string());
    }

    fn get_queue_size(&self) -> usize {
        (*self.job_queue).read().unwrap().len()
    }

    fn browser_drag_request(&self, dragged_url: &str) {
        let errors_rep = ErrorRepo::by_connection((*self.erro_repo).borrow().get_connection());
        let gp_sender: Sender<Job> = self.gp_job_sender.as_ref().unwrap().clone();
        let drag_i = DragInner::new(
            dragged_url.to_string(),
            self.source_c_sender.as_ref().unwrap().clone(),
            self.web_fetcher.clone(),
            errors_rep,
            gp_sender,
        );
        self.add_to_queue(DLJob::BrowserDragEvaluation(drag_i));
    }

    fn launch_webbrowser(&self, url: String, cl_id: isize, list_pos: u32) {
        let cl_sender: Sender<CJob> = self.contentlist_job_sender.as_ref().unwrap().clone();
        let inner = LaunchInner::new(url, cl_id, list_pos, cl_sender);
        self.add_to_queue(DLJob::LaunchWebBrowser(inner));
    }

    fn get_statistics(&self) -> [u32; DLKIND_MAX] {
        *self.call_statistic.borrow()
    }
}

impl Buildable for Downloader {
    type Output = Downloader;
    fn build(conf: Box<dyn BuildConfig>, ac: &AppContext) -> Self::Output {
        let mut dl = Downloader::new_ac(ac);
        if let Some(n) = conf.get_int(CONF_DOWNLOADER_THREADS) {
            dl.config.num_downloader_threads = n as u8;
        } else {
            dl.config.num_downloader_threads = 1;
        }
        dl
    }
}

impl StartupWithAppContext for Downloader {
    fn startup(&mut self, ac: &AppContext) {
        let fceedcontents_r: Rc<RefCell<dyn IContentList>> = ac.get_rc::<FeedContents>().unwrap();
        let cjob_sender = (*fceedcontents_r).borrow().get_job_sender();
        self.contentlist_job_sender = Some(cjob_sender);
        let stc_r: Rc<RefCell<dyn ISourceTreeController>> =
            ac.get_rc::<SourceTreeController>().unwrap();
        let sjob_sender = (*stc_r).borrow().get_job_sender();
        self.source_c_sender = Some(sjob_sender);
        let gp_r: Rc<RefCell<GuiProcessor>> = ac.get_rc::<GuiProcessor>().unwrap();
        let gp_job_sender = (*gp_r).borrow().get_job_sender();
        self.gp_job_sender = Some(gp_job_sender);
        let timer_r = ac.get_rc::<Timer>().unwrap();
        let dl_r = ac.get_rc::<Downloader>().unwrap();
        (*timer_r)
            .borrow_mut()
            .register(&TimerEvent::Shutdown, dl_r, true);
        self.startup();
    }
}

impl TimerReceiver for Downloader {
    fn trigger_mut(&mut self, event: &TimerEvent) {
        if event == &TimerEvent::Shutdown {
            self.shutdown();
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub num_downloader_threads: u8,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            num_downloader_threads: 1,
        }
    }
}
