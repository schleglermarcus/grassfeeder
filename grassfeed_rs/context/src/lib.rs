pub mod appcontext;
pub mod buildconfig;

use appcontext::AppContext;
use std::cell::RefCell;
use std::rc::Rc;

// ---- Context

pub trait Buildable: Sized {
    type Output;
    fn build(conf: Box<dyn BuildConfig>, appcontext: &AppContext) -> Self::Output;
}

pub trait BuildConfig {
    fn get(&self, key: &str) -> Option<String>;
    fn get_int(&self, key: &str) -> Option<isize>;
    fn get_bool(&self, key: &str) -> bool;
    fn dump(&self);
}

/// Phase 2
pub trait StartupWithAppContext {
    fn startup(&mut self, _ac: &AppContext) {} // default impl does nothing
}

// ---- Timer

#[repr(u64)]
#[allow(unused)]
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum TimerEvent {
    Timer100s = 0,
    Timer10s,
    Timer1s,
    Timer200ms,
    Timer100ms,
    Timer50ms,
    Timer20ms,
    Timer10ms,
    Shutdown,
    Startup,
    #[default]
    None,
}

pub const TIMER_EVENT_TABLE: [(TimerEvent, u64); 11] = [
    (TimerEvent::Timer100s, 100000),
    (TimerEvent::Timer10s, 10000),
    (TimerEvent::Timer1s, 1000),
    (TimerEvent::Timer200ms, 200),
    (TimerEvent::Timer100ms, 100),
    (TimerEvent::Timer20ms, 20),
    (TimerEvent::Timer50ms, 50),
    (TimerEvent::Timer10ms, 10),
    (TimerEvent::Shutdown, 0),
    (TimerEvent::Startup, 0),
    (TimerEvent::None, 0),
];

impl TimerEvent {
    pub fn from_int(i: usize) -> TimerEvent {
        TIMER_EVENT_TABLE[i].0.clone()
    }
    pub fn delay(i: usize) -> u64 {
        TIMER_EVENT_TABLE[i].1
    }
    pub fn len() -> usize {
        TIMER_EVENT_TABLE.len()
    }
}

pub trait TimerRegistry {
    fn register(
        &mut self,
        te: &TimerEvent,
        observer: Rc<RefCell<dyn TimerReceiver + 'static>>,
        call_mutable: bool,
    );

    fn notify_all(&self, te: &TimerEvent);
}

pub trait TimerReceiver {
    fn trigger_mut(&mut self, _ev: &TimerEvent) {
        panic!("TimerReceiver-mut registered but not implemented!")
    }
    fn trigger(&self, _ev: &TimerEvent) {
        panic!("TimerReceiver immutable registered but not implemented!")
    }
}
