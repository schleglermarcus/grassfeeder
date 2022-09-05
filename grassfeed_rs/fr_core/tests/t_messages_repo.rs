#[ignore]
#[test]
fn t_() {
    setup();

    let msgrepo_r = prepare_3_rows();
    let mut e1 = MessageRow::default();
    e1.feed_src_id = 1;
    let _r = (*msgrepo_r).borrow().insert(&e1);

    let all = (*msgrepo_r).borrow().get_all_messages();
    for msg in all {
        trace!("{:?}", msg);
    }
    let src_not: Vec<i32> = vec![0, 3];
    let msg_not = (*msgrepo_r).borrow().get_src_not_contained(&src_not);
	for msg in & msg_not {
        debug!(" NOT {:?}", msg);
    }
    assert_eq!(msg_not.len(), 1);
    assert_eq!(msg_not.get(0).unwrap().feed_src_id, 1);
}

use fr_core::db::message::MessageRow;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use std::cell::RefCell;
use std::rc::Rc;

fn prepare_3_rows() -> Rc<RefCell<dyn IMessagesRepo>> {
    setup();
    let messagesrepo = MessagesRepo::new(":memory:".to_string());
    messagesrepo.get_ctx().create_table();
    let msg_r: Rc<RefCell<dyn IMessagesRepo>> = Rc::new(RefCell::new(messagesrepo));
    let mut e1 = MessageRow::default();
    let _r = (*msg_r).borrow().insert(&e1);
    e1.feed_src_id = 3;
    let _r = (*msg_r).borrow().insert(&e1);
    e1.feed_src_id = 3;
    e1.is_read = true;
    let _r = (*msg_r).borrow().insert(&e1);
    msg_r
}

// ------------------------------------

#[allow(unused_imports)]
#[macro_use]
extern crate log;
mod logger_config;
use std::sync::Once;
static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = logger_config::setup_fern_logger(0);
    });
}
