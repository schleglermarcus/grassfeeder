use crate::config::configmanager::ConfigManager;
use crate::controller::contentlist::CJob;
use crate::controller::contentlist::FeedContents;
use crate::controller::contentlist::IFeedContents;
use crate::controller::guiprocessor::GuiProcessor;
use crate::controller::guiprocessor::Job;
use crate::controller::sourcetree::ISourceTreeController;
use crate::controller::sourcetree::SJob;
use crate::controller::sourcetree::SourceTreeController;
use crate::db::icon_repo::IconRepo;
use crate::db::messages_repo::MessagesRepo;
use crate::db::subscription_repo::ISubscriptionRepo;
use crate::db::subscription_repo::SubscriptionRepo;
use crate::downloader::comprehensive::ComprStart;
use crate::downloader::comprehensive::ComprehensiveInner;
use crate::downloader::db_clean::CleanerInner;
use crate::downloader::db_clean::CleanerStart;
use crate::downloader::icons::IconInner;
use crate::downloader::icons::IconLoadStart;
use crate::downloader::messages::FetchInner;
use crate::downloader::messages::FetchStart;
use crate::timer::Timer;
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
pub const DOWNLOADER_LOOP_DELAY_S: u8 = 1;
pub const DOWNLOADER_JOB_QUEUE: usize = 10000;

pub trait IDownloader {
    fn shutdown(&mut self);
    fn is_running(&self) -> bool;
    fn get_config(&self) -> Config;
    fn set_conf_num_threads(&mut self, n: u8);
    fn is_dl_busy(&self) -> [u8; DOWNLOADER_MAX_NUM_THREADS];
    fn add_update_source(&self, f_source_repo_id: isize);
    fn new_feedsource_request(&self, fs_edit_url: &str);
    fn load_icon(&self, fs_id: isize, fs_url: String, old_icon_id: usize);
    fn cleanup_db(&self);
    fn get_queue_size(&self) -> usize;
}

#[derive(Debug, PartialEq)]
pub enum DLJob {
    None, // to trigger the loop only
    Feed(FetchInner),
    Icon(IconInner),
    ComprehensiveFeed(ComprehensiveInner),
    CleanDatabase(CleanerInner),
}

trait DLKind {
    fn kind(&self) -> u8;
}

impl DLKind for DLJob {
    /// see  [crate::guiprocessor::dl_char_for_kind ]
    fn kind(&self) -> u8 {
        match self {
            DLJob::None => 0,
            DLJob::Feed(_) => 1,
            DLJob::Icon(_) => 2,
            DLJob::ComprehensiveFeed(_) => 3,
            DLJob::CleanDatabase(_) => 4,
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
    pub busy_indicators: Arc<RwLock<[u8; DOWNLOADER_MAX_NUM_THREADS as usize]>>,
    messagesrepo: Rc<RefCell<MessagesRepo>>,
    job_queue: Arc<RwLock<VecDeque<DLJob>>>,
}

impl Downloader {
    pub fn new(
        fetcher: WebFetcherType,
        subscr_r: Rc<RefCell<dyn ISubscriptionRepo>>,
        icon_repo_r: Rc<RefCell<IconRepo>>,
        cm_r: Rc<RefCell<ConfigManager>>,
        msgrepo: Rc<RefCell<MessagesRepo>>,
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
            busy_indicators: Arc::new(RwLock::new([0; DOWNLOADER_MAX_NUM_THREADS as usize])),
            messagesrepo: msgrepo,
            job_queue: Arc::new(RwLock::new(VecDeque::default())),
        }
    }

    pub fn new_ac(ac: &AppContext) -> Self {
        let fetcher: WebFetcherType = Arc::new(Box::new(HttpFetcher {}));
        let subscr_r: Rc<RefCell<dyn ISubscriptionRepo>> =
            (*ac).get_rc::<SubscriptionRepo>().unwrap();
        let iconrepo_r: Rc<RefCell<IconRepo>> = (*ac).get_rc::<IconRepo>().unwrap();
        let cm_r = (*ac).get_rc::<ConfigManager>().unwrap();
        let msgrepo = (*ac).get_rc::<MessagesRepo>().unwrap();
        Downloader::new(fetcher, subscr_r, iconrepo_r, cm_r, msgrepo)
    }

    pub fn startup(&mut self) {
        if self.config.num_downloader_threads == 0
            || self.config.num_downloader_threads > DOWNLOADER_MAX_NUM_THREADS as u8
        {
            warn!(
                "Invalid Number of downloader threads: {}",
                self.config.num_downloader_threads
            );
            self.config.num_downloader_threads = 1;
        }
        for n in 0..self.config.num_downloader_threads {
            let gp_sender: Sender<Job> = self.gp_job_sender.as_ref().unwrap().clone();
            let queue_a = self.job_queue.clone();
            // let f_q_r = self.fetch_queue_receiver.clone();
            let busy_a = self.busy_indicators.clone();
            let builder = thread::Builder::new().name(format!("dl_{}", n));
            let h = builder
                .spawn(move || loop {
                    let queue_size = (*queue_a).read().unwrap().len();
                    let o_job = (*queue_a).write().unwrap().pop_front();
                    if let Some(dljob) = o_job {
                        let job_kind = dljob.kind();
                        (*busy_a).write().unwrap()[n as usize] = job_kind;
                        let _r = gp_sender.send(Job::DownloaderJobStarted(n as u8, job_kind));
                        Self::process_job(dljob, queue_size);
                        let _r = gp_sender.send(Job::DownloaderJobFinished(n as u8, job_kind));
                        (*busy_a).write().unwrap()[n as usize] = 0;
                    }
                    let k = KEEPRUNNING.load(Ordering::Relaxed);
                    if k {
                        thread::sleep(Duration::from_millis(100));
                    } else {
                        break;
                    }
                })
                .unwrap();
            self.joinhandles.push(h);
        }
    }

    fn add_to_queue(&self, dljob: DLJob) {
        let contains = (*self.job_queue).read().unwrap().contains(&dljob);
        if contains {
            let kind = dljob.kind();
            debug!("download job already queued:  {}:{:?}", kind, &dljob);
        } else {
            (*self.job_queue).write().unwrap().push_back(dljob);
        }
    }

    fn process_job(dljob: DLJob, queue_size: usize) {
        let now = std::time::Instant::now();
        let job_description = format!("{:?}", &dljob);

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
        }
        let elapsedms = now.elapsed().as_millis();
        let t_name: String = std::thread::current().name().unwrap().to_string();
        if elapsedms > 3000 {
            trace!(
                "{} {:?} took {}ms   #Q={}",
                t_name,
                job_description,
                elapsedms,
                queue_size
            );
        }
    }
}

impl IDownloader for Downloader {
    fn is_dl_busy(&self) -> [u8; DOWNLOADER_MAX_NUM_THREADS] {
        *(*self.busy_indicators).read().unwrap()
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
        let subscription_repo =
            SubscriptionRepo::by_existing_list((*self.subscriptionrepo_r).borrow().get_list());
        let icon_repo = IconRepo::by_existing_list((*self.iconrepo_r).borrow().get_list());

        let msgrepo = MessagesRepo::new_by_connection(
            (*self.messagesrepo).borrow().get_ctx().get_connection(),
        );
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
        };
        self.add_to_queue(DLJob::Feed(new_fetch_job));
    }

    fn load_icon(&self, fs_id: isize, fs_url: String, old_icon_id: usize) {
        let icon_repo = IconRepo::by_existing_list((*self.iconrepo_r).borrow().get_list());
        let dl_inner = IconInner {
            fs_repo_id: fs_id,
            feed_url: fs_url,
            icon_url: String::default(),
            iconrepo: icon_repo,
            web_fetcher: self.web_fetcher.clone(),
            download_error_happened: false,
            icon_bytes: Vec::default(),
            fs_icon_id_old: old_icon_id as isize,
            sourcetree_job_sender: self.source_c_sender.as_ref().unwrap().clone(),
            feed_homepage: String::default(),
            feed_download_text: String::default(),
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
        let subs_repo =
            SubscriptionRepo::by_existing_list((*self.subscriptionrepo_r).borrow().get_list());
        let msgrepo1 = MessagesRepo::new_by_connection(
            (*self.messagesrepo).borrow().get_ctx().get_connection(),
        );
        let cleaner_i = CleanerInner::new(
            self.contentlist_job_sender.as_ref().unwrap().clone(),
            self.source_c_sender.as_ref().unwrap().clone(),
            subs_repo,
            msgrepo1,
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
        // (*self.configmanager_r).borrow_mut().set_section_key(
        //     &Self::section_name(),
        //     CONF_DOWNLOADER_THREADS,
        //     n.to_string().as_str(),
        // );
        (*self.configmanager_r)
            .borrow()
            .set_val(CONF_DOWNLOADER_THREADS, n.to_string());
    }

    fn get_queue_size(&self) -> usize {
        // self.fetch_queue_sender.len()
        (*self.job_queue).read().unwrap().len()
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

    fn section_name() -> String {
        String::from("contentdownloader")
    }
}

impl StartupWithAppContext for Downloader {
    fn startup(&mut self, ac: &AppContext) {
        let fceedcontents_r: Rc<RefCell<dyn IFeedContents>> = ac.get_rc::<FeedContents>().unwrap();
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
            .register(&TimerEvent::Shutdown, dl_r);
        self.startup();
    }
}

impl TimerReceiver for Downloader {
    fn trigger(&mut self, event: &TimerEvent) {
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

// ---

#[cfg(test)]
mod downloader_test {
    //  use super::*;
}
