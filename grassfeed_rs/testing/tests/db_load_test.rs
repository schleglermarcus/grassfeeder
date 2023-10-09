use fr_core::db::message::MessageRow;
use fr_core::db::messages_repo::IMessagesRepo;
use fr_core::db::messages_repo::MessagesRepo;
use rand::Rng;
use rusqlite::Connection;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;

/*
13:05:46:982 DEBUG _entries_filter	num 100	 	used 14ms
13:05:47:024 DEBUG _entries_filter	num 1000	 	used 42ms
13:05:47:234 DEBUG _entries_filter	num 10000	 	used 209ms
13:05:50:537 DEBUG _entries_filter	num 100000	 	used 3303ms
*/
#[ignore]
#[test]
fn db_some_threads() {
    setup();

    let mr = MessagesRepo::new_in_mem();
    mr.get_ctx().create_table();
    let conn_a = mr.get_ctx().get_connection();

    let counts: [usize; 4] = [10, 100, 1000, 10000];
    counts.iter().for_each(|count| {
        let now = std::time::Instant::now();
        let t = 10;

        fill_sq(conn_a.clone(), t, *count);
        debug!("num {}	 \tused {}ms", (t * count), now.elapsed().as_millis());
    });
}

fn fill_sq(conn_a: Arc<Mutex<Connection>>, num_threads: usize, num_entries: usize) {
    let mr_i = MessagesRepo::new_by_connection(conn_a.clone());
    let _r = mr_i.get_ctx().delete_table();
    mr_i.get_ctx().create_table();
    //trace!(        "starting insert threads:{} lines:{}",        num_threads, num_entries    );
    let handles: Vec<JoinHandle<()>> = (0..num_threads)
        .into_iter()
        .map(|n| {
            let mr_i = MessagesRepo::new_by_connection(conn_a.clone());
            std::thread::Builder::new()
                .name(format!("T{}", n))
                .spawn(move || {
                    insert_rows_tx(&mr_i, num_entries);
                })
                .unwrap()
        })
        .collect();
    handles.into_iter().for_each(|h| {
        let _r = h.join();
    });
    assert_eq!(mr_i.get_all_sum() as usize, num_entries * num_threads);
}

fn insert_rows_tx(repo: &MessagesRepo, num_entries: usize) {
    let names: Vec<String> = create_random_names(num_entries);
    let fc_list: Vec<MessageRow> = names
        .iter()
        .map(|n| {
            let mut fce = MessageRow::default();
            fce.title = n.clone();
            fce
        })
        .collect::<Vec<MessageRow>>();
    let _r = repo.insert_tx(&fc_list);
}

fn get_random_name() -> String {
    let mut rng = rand::thread_rng();
    let mut arr: Vec<u8> = vec![32; 10];
    let _r = rng.try_fill(&mut arr[..]);
    base64::encode(arr)
}

fn create_random_names(num_elements: usize) -> Vec<String> {
    let randomnames = (0..num_elements)
        .map(|_n| get_random_name())
        .collect::<Vec<String>>();
    randomnames
}

// ------------------------------------

#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = testing::logger_config_local::setup_logger();
    });
}
