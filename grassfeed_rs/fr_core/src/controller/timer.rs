use context::appcontext::AppContext;
use context::BuildConfig;
use context::Buildable;
use context::StartupWithAppContext;
use context::TimerEvent;
use context::TimerReceiver;
use context::TimerRegistry;
use context::TIMER_EVENT_TABLE;
use flume::Receiver;
use flume::Sender;
use std::cell::RefCell;
use std::rc::Rc;
use std::rc::Weak;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const MAINLOOP_SLEEP_MS: u64 = 20; // 50
const TIMER_JOB_QUEUE_SIZE: usize = 2;

pub trait KeepRunningNotify {
    fn keep_running(&self) -> bool;
}

pub enum TimerJob {
    Shutdown,
}

pub trait ITimer: TimerRegistry {
    fn main_loop(&mut self);
    fn get_ctrl_sender(&self) -> Sender<TimerJob>;
}

pub struct Timer {
    schedules: [TimerSchedule; TIMER_EVENT_TABLE.len()],
    timer_receiver: Receiver<TimerJob>,
    timer_sender: Sender<TimerJob>,
    signal_term: Arc<AtomicBool>,
    signal_int: Arc<AtomicBool>,
}

impl Timer {}

pub fn build_timer() -> Timer {
    let (t_s, t_r) = flume::bounded::<TimerJob>(TIMER_JOB_QUEUE_SIZE);
    let sig_term_a = Arc::new(AtomicBool::new(false));
    let sig_int_a = Arc::new(AtomicBool::new(false));

    //  Later  add the term hook    only without debug
    let _r = signal_hook::flag::register(signal_hook::consts::SIGTERM, sig_term_a.clone());
    let _r = signal_hook::flag::register(signal_hook::consts::SIGINT, sig_term_a.clone());

    Timer {
        schedules: Default::default(),
        timer_sender: t_s,
        timer_receiver: t_r,
        signal_term: sig_term_a,
        signal_int: sig_int_a,
    }
}

impl ITimer for Timer {
    fn main_loop(&mut self) {
        let mut keeprunning = true;
        let start_time = Instant::now();
        self.notify_all(&TimerEvent::Startup);
        while keeprunning {
            thread::sleep(Duration::from_millis(MAINLOOP_SLEEP_MS));
            let elapsed_ms = Instant::now().duration_since(start_time).as_millis();
            for (i, _t_ev) in TIMER_EVENT_TABLE
                .iter()
                .enumerate()
                .take(self.schedules.len())
            {
                if _t_ev.1 == 0 {
                    continue;
                }
                let te: TimerEvent = TimerEvent::from_int(i);
                if elapsed_ms > self.schedules[i].next_trigger_ms {
                    let next_trigger = elapsed_ms + TimerEvent::delay(i) as u128;
                    self.schedules[i].next_trigger_ms = next_trigger;
                    self.notify_all(&te);
                }
            }
            if let Ok(job) = self.timer_receiver.try_recv() {
                match job {
                    TimerJob::Shutdown => {
                        keeprunning = false;
                        self.notify_all(&TimerEvent::Shutdown);
                    }
                }
            }
            if self.signal_term.load(Ordering::Relaxed) || self.signal_int.load(Ordering::Relaxed) {
                info!("got signal TERM or INT: shutdown");
                keeprunning = false;
                self.notify_all(&TimerEvent::Shutdown);
            }
        }
    }

    fn get_ctrl_sender(&self) -> Sender<TimerJob> {
        self.timer_sender.clone()
    }
}

#[derive(Default, Clone)]
struct TimerSchedule {
    next_trigger_ms: u128,
    receivers: Vec<Weak<RefCell<dyn TimerReceiver + 'static>>>,
}

impl TimerRegistry for Timer {
    fn register(&mut self, te: &TimerEvent, observer: Rc<RefCell<dyn TimerReceiver + 'static>>) {
        let index = te.clone() as usize;
        if index >= TimerEvent::len() {
            warn!("unknown event={:?}  ", &te);
            return;
        }
        self.schedules[index]
            .receivers
            .push(Rc::downgrade(&observer));
    }

    fn notify_all(&self, te: &TimerEvent) {
        let index = te.clone() as usize;
        if index >= TimerEvent::len() {
            error!("notify_all: unknown event={:?}  ", &te);
            return;
        }
        self.schedules[index]
            .receivers
            .iter()
            .enumerate()
            .for_each(|(_n, r)| {
                if let Some(rc) = r.upgrade() {
                    (*rc).borrow_mut().trigger(te);
                }
            });
    }
}

impl Buildable for Timer {
    type Output = Timer;
    fn build(_conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
        build_timer()
    }
}

impl StartupWithAppContext for Timer {}

//------------------------------------------------------

#[cfg(test)]
mod appcontext_test {

    use crate::config::configmanager::ConfigManager;
    use crate::config::init_system::create_system_config;
    use crate::config::init_system::GrassFeederConfig;
    use crate::controller::timer::ITimer;
    use crate::controller::timer::Timer;
    use crate::controller::timer::TimerJob;
    use context::appcontext::AppContext;
    use context::BuildConfig;
    use context::Buildable;
    use context::StartupWithAppContext;
    use context::TimerEvent;
    use context::TimerReceiver;
    use flume::Sender;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;
    pub static DBU_IN_USE: AtomicBool = AtomicBool::new(false);

    //----
    struct GUIP {
        timer_r: Rc<RefCell<dyn ITimer>>,
        _dbu_r: Rc<RefCell<DBU>>,
        timer_sender: Option<Sender<TimerJob>>,
    }

    impl StartupWithAppContext for GUIP {
        fn startup(&mut self, ac: &AppContext) {
            let gp_r = ac.get_rc::<GUIP>().unwrap();
            {
                let mut timer = (*self.timer_r).borrow_mut();
                timer.register(&TimerEvent::Timer100ms, gp_r);
                self.timer_sender = Some(timer.get_ctrl_sender());
            }
        }
    }

    impl TimerReceiver for GUIP {
        fn trigger(&mut self, _event: &TimerEvent) {}
    }

    impl Buildable for GUIP {
        type Output = GUIP;
        fn build(_conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
            GUIP::new(_appcontext)
        }
    }

    #[derive(Debug, Clone)]
    pub enum Job {}

    impl GUIP {
        pub fn new(ac: &AppContext) -> Self {
            let (_q_s, _q_r) = flume::bounded::<Job>(1);
            GUIP {
                timer_r: (*ac).get_rc::<Timer>().unwrap(),
                _dbu_r: (*ac).get_rc::<DBU>().unwrap(),
                timer_sender: None,
            }
        }
    }

    //----
    struct DBU {}
    impl DBU {}
    impl Buildable for DBU {
        type Output = DBU;

        fn build(_conf: Box<dyn BuildConfig>, _appcontext: &AppContext) -> Self::Output {
            DBU_IN_USE.store(true, Ordering::Relaxed);
            DBU {}
        }
    }
    impl StartupWithAppContext for DBU {
        fn startup(&mut self, _ac: &AppContext) {}
    }

    impl Drop for DBU {
        fn drop(&mut self) {
            DBU_IN_USE.store(false, Ordering::Relaxed);
        }
    }

    fn run_example() {
        let gfc = GrassFeederConfig {
            path_config: "../target/db_timer_uninit".to_string(),
            path_cache: "../target/db_timer_uninit".to_string(),
            debug_mode: true,
            version: "test_timer_uninit".to_string(),
        };
        let systemconf = create_system_config(&gfc);
        let mut appcontext = AppContext::new(systemconf);
        appcontext.build::<ConfigManager>();
        appcontext.build::<Timer>();
        appcontext.build::<DBU>();
        appcontext.build::<GUIP>();
        appcontext.startup();
    }

    #[test]
    fn test_timer_uninit() {
        run_example();
        assert_eq!(DBU_IN_USE.load(Ordering::Relaxed), false);
        run_example();
        assert_eq!(DBU_IN_USE.load(Ordering::Relaxed), false);
    }
}
