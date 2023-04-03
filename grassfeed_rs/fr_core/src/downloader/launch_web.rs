use crate::controller::contentlist::CJob;
use crate::util::Step;
use crate::util::StepResult;
use flume::Sender;
use std::time::Duration;

const LAUNCH_WAIT_MS: u64 = 100;

pub struct LaunchInner {
    pub url: String,
    pub cl_id: isize,
    pub list_pos: u32,
    gp_sender: Sender<CJob>,
}

impl LaunchInner {
    pub fn new(url_: String, cl_id_: isize, list_pos_: u32, gp_s: Sender<CJob>) -> Self {
        LaunchInner {
            url: url_,
            cl_id: cl_id_,
            list_pos: list_pos_,
            gp_sender: gp_s,
        }
    }
}

impl std::fmt::Debug for LaunchInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("").field("url", &self.url).finish()
    }
}

impl PartialEq for LaunchInner {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

pub struct LaunchWebBrowserStart(LaunchInner);
impl LaunchWebBrowserStart {
    pub fn new(i: LaunchInner) -> Self {
        LaunchWebBrowserStart(i)
    }
}

impl Step<LaunchInner> for LaunchWebBrowserStart {
    fn step(self: Box<Self>) -> StepResult<LaunchInner> {
        let r = webbrowser::open(&self.0.url);
        if r.is_err() {
            warn!("webbrowser::open {:?} {:?} ", &self.0.url, r);
            return StepResult::Stop(self.0);
        }
        std::thread::sleep(Duration::from_millis(LAUNCH_WAIT_MS));
        StepResult::Continue(Box::new(LaunchNotify(self.0)))
    }
}

pub struct LaunchNotify(LaunchInner);
impl Step<LaunchInner> for LaunchNotify {
    fn step(self: Box<Self>) -> StepResult<LaunchInner> {
        let _r = self
            .0
            .gp_sender
            .send(CJob::LaunchBrowserSuccess(self.0.cl_id, self.0.list_pos));
        StepResult::Stop(self.0)
    }
}
