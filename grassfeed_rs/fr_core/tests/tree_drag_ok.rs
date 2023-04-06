//  cargo watch -s "cargo test  --test tree_drag_ok -- --test-threads 1 "
mod downloader_dummy;
mod tree_drag_common;

use crate::tree_drag_common::dataset_simple_trio;
use crate::tree_drag_common::dataset_some_tree;
use crate::tree_drag_common::dataset_three_folders;
use crate::tree_drag_common::prepare_subscription_move;
use fr_core::controller::subscriptionmove::ISubscriptionMove;
use fr_core::db::subscription_entry::SubscriptionEntry;

/// Dragging the first folder   between the second and third.   0 -> 2
//  #[ignore]
#[test]
fn drag_folder_one_down() {
    setup(); //
    let fs_list: Vec<SubscriptionEntry> = dataset_three_folders();
    let (fsc, r_fsource) = prepare_subscription_move(fs_list);
    match fsc.drag_calc_positions(&vec![0], &vec![2]) {
        Ok((from_entry, to_parent_id, to_folderpos)) => {
            // debug!(                "OK:  to_parent_id={}  to_folderpos={} ",                to_parent_id, to_folderpos            );
            assert_eq!(to_folderpos, 2);
            fsc.drag_move(from_entry, to_parent_id, to_folderpos);
        }
        Err(ref e) => {
            error!("{:?}", e);
            assert!(false);
        }
    }
    let result: Vec<SubscriptionEntry> = (*r_fsource).borrow().get_all_entries();
    assert_eq!(result.len(), 3);
    //  for e in &result {        debug!("{:?}", &e);    }
    assert_eq!(result[1].folder_position, 0);
    assert_eq!(result[0].folder_position, 1);
    assert_eq!(result[2].folder_position, 2);
}

#[test]
fn drag_different_parent_down() {
    setup();
    let mut fs_list: Vec<SubscriptionEntry> = Vec::default();
    let mut fse = SubscriptionEntry::from_new_url("feed1-display".to_string(), String::default());
    fse.subs_id = 1;
    fse.folder_position = 0;
    fs_list.push(fse.clone());
    fse.display_name = "folder2".to_string();
    fse.subs_id = 2;
    fse.folder_position = 1;
    fse.is_folder = true;
    fs_list.push(fse.clone());
    let (fsc, r_fsource) = prepare_subscription_move(fs_list);
    match fsc.drag_calc_positions(&vec![0], &vec![1, 0]) {
        Ok((from_entry, to_parent_id, to_folderpos)) => {
            fsc.drag_move(from_entry, to_parent_id, to_folderpos);
        }
        Err(ref e) => {
            error!("{:?}", e);
            assert!(false);
        }
    }
    //    r_fsource.borrow().debug_dump_tree("DIF_");
    let result: Vec<SubscriptionEntry> = (*r_fsource).borrow().get_by_parent_repo_id(2);
    assert_eq!(result.len(), 1);
}

#[test]
fn drag_outofrange_fail() {
    setup();
    let fs_list: Vec<SubscriptionEntry> = dataset_simple_trio();
    let (fsc, _r_fsource) = prepare_subscription_move(fs_list);
    match fsc.drag_calc_positions(&vec![0], &vec![0, 6, 0, 0]) {
        Ok(_) => assert!(false),
        Err(_e) => {
            //  trace!("{:?}", e);
        }
    }
}

#[test]
fn drag_folder_one_up() {
    setup(); //
    let fs_list: Vec<SubscriptionEntry> = dataset_three_folders();
    let (fsc, r_fsource) = prepare_subscription_move(fs_list);
    let success = fsc.on_subscription_drag(0, vec![1], vec![0, 0]);
    // r_fsource.borrow().debug_dump_tree("after ");
    let result: Vec<SubscriptionEntry> = (*r_fsource).borrow().get_by_parent_repo_id(1);
    assert_eq!(result.len(), 1);
    assert!(success);
}

#[test]
fn same_folder_move_third_under_first() {
    setup(); //   [2] => [1]
    let fs_list: Vec<SubscriptionEntry> = dataset_simple_trio();
    let (fsc, r_fsource) = prepare_subscription_move(fs_list);
    let entries: Vec<SubscriptionEntry> = (*r_fsource).borrow().get_by_parent_repo_id(0);
    fsc.drag_move(entries[2].clone(), 0, 1);
    let result: Vec<SubscriptionEntry> = (*r_fsource).borrow().get_by_parent_repo_id(0);
    assert_eq!(result.len(), 3);
    assert_eq!(result.get(0).unwrap().subs_id, 1);
    assert_eq!(result.get(1).unwrap().subs_id, 3);
    assert_eq!(result.get(2).unwrap().subs_id, 2);
}

#[test]
fn same_folder_move_first_under_second() {
    setup();
    let fs_list: Vec<SubscriptionEntry> = dataset_simple_trio();
    let (fsc, r_fsource) = prepare_subscription_move(fs_list);
    let entries: Vec<SubscriptionEntry> = (*r_fsource).borrow().get_by_parent_repo_id(0);
    fsc.drag_move(entries[0].clone(), 0, 2);
    //    r_fsource.borrow().debug_dump_tree("UNS_");
    let result: Vec<SubscriptionEntry> = (*r_fsource).borrow().get_by_parent_repo_id(0);
    assert_eq!(result.len(), 3);
    assert_eq!(result.get(0).unwrap().subs_id, 2);
    assert_eq!(result.get(1).unwrap().subs_id, 1);
    assert_eq!(result.get(2).unwrap().subs_id, 3);
}

// -------------------

// drag onto first folder, shall put it on very top

#[test]
fn drag_2nd_folder_to_1st_folder() {
    setup();
    let fs_list: Vec<SubscriptionEntry> = dataset_some_tree();
    let (fsc, r_fsource) = prepare_subscription_move(fs_list);
    match fsc.drag_calc_positions(&vec![1], &vec![0, 0]) {
        Ok((from_entry, to_parent_id, to_folderpos)) => {
            // debug!(                "to_parent_id={}, to_folderpos={} ",                to_parent_id, to_folderpos            );
            assert_eq!(to_parent_id, 1);
            assert_eq!(to_folderpos, 0);
            fsc.drag_move(from_entry, to_parent_id, to_folderpos);
        }
        Err(e) => error!("{:?}", e),
    }
    // r_fsource.borrow().debug_dump_tree("\nup2 ");
    let result: Vec<SubscriptionEntry> = (*r_fsource).borrow().get_by_parent_repo_id(0);
    assert_eq!(result.len(), 1);
    assert_eq!(result.get(0).unwrap().subs_id, 1);
}

// drag to root, shall put it on very top
#[test]
fn drag_up_to_root() {
    setup();
    let fs_list: Vec<SubscriptionEntry> = dataset_some_tree();
    let (fsc, r_fsource) = prepare_subscription_move(fs_list);
    match fsc.drag_calc_positions(&vec![0, 1], &vec![0]) {
        Ok((from_entry, to_parent_id, to_folderpos)) => {
            // debug!(                "to_parent_id={}, to_folderpos={} ",                to_parent_id, to_folderpos            );
            assert_eq!(to_parent_id, 0);
            assert_eq!(to_folderpos, 0);
            fsc.drag_move(from_entry, to_parent_id, to_folderpos);
        }
        Err(e) => error!("{:?}", e),
    }
    // r_fsource.borrow().debug_dump_tree("\nup2 ");
    let result: Vec<SubscriptionEntry> = (*r_fsource).borrow().get_by_parent_repo_id(0);
    assert_eq!(result.len(), 3);
    assert_eq!(result.get(0).unwrap().subs_id, 3);
    assert_eq!(result.get(1).unwrap().subs_id, 1);
}

//  may not drag the folder under the feed
#[test]
fn drag_under_feed() {
    setup();
    let fs_list: Vec<SubscriptionEntry> = dataset_some_tree();
    let (fsc, _r_fsource) = prepare_subscription_move(fs_list);
    match fsc.drag_calc_positions(&vec![1], &vec![0, 0, 0]) {
        Ok((from_entry, to_parent_id, to_folderpos)) => {
            // fsc.drag_move(from_entry, to_parent_id, to_folderpos);
            panic!(
                "may not drag onto entry {:?} => P{} FP{}",
                from_entry, to_parent_id, to_folderpos
            );
        }
        Err(_e) => {}
    }
}

#[test]
fn drag_folder_on_other_folder() {
    setup();
    let fs_list: Vec<SubscriptionEntry> = dataset_three_folders();
    let (fsc, r_fsource) = prepare_subscription_move(fs_list);
    //  drag the first entry onto second
    match fsc.drag_calc_positions(&vec![0], &vec![1, 0]) {
        Ok((from_entry, to_parent_id, to_folderpos)) => {
            fsc.drag_move(from_entry, to_parent_id, to_folderpos);
        }
        Err(e) => {
            error!("{:?}", e);
        }
    }
    //    r_fsource.borrow().debug_dump_tree("\nontosecond2: ");
    let result: Vec<SubscriptionEntry> = (*r_fsource).borrow().get_by_parent_repo_id(0);
    assert_eq!(result.len(), 2);
    assert_eq!(result.get(0).unwrap().subs_id, 2);
    assert_eq!(result.get(1).unwrap().subs_id, 3);
}

// This is a non-drag event:  dragging a folder to itself.  Gtk delivers:  [0] => [0, 1]
// It shall deliver an error message
#[test]
fn reject_folder_onto_child() {
    setup();
    let fs_list: Vec<SubscriptionEntry> = dataset_some_tree();
    let (fsc, _r_fsource) = prepare_subscription_move(fs_list);
    match fsc.drag_calc_positions(&vec![0], &vec![0, 1]) {
        Ok((_from_entry, _to_parent_id, _to_folderpos)) => {
            assert!(false);
        }
        Err(ref _e) => {}
    }
}

#[test]
fn drag_entry_below_last() {
    setup();
    let fs_list: Vec<SubscriptionEntry> = dataset_simple_trio();
    let (fsc, r_fsource) = prepare_subscription_move(fs_list);
    match fsc.drag_calc_positions(&vec![0], &vec![3]) {
        Ok((from_entry, to_parent_id, to_folderpos)) => {
            fsc.drag_move(from_entry, to_parent_id, to_folderpos);
        }
        Err(e) => {
            warn!("{:?}", e);
        }
    }
    //  r_fsource.borrow().debug_dump_tree("\nbelow2: ");
    let result: Vec<SubscriptionEntry> = (*r_fsource).borrow().get_by_parent_repo_id(0);
    assert_eq!(result.len(), 3);
    assert_eq!(result.get(0).unwrap().subs_id, 2); //
}

// shall not drag, since the destination is an entry
//  drag the first entry onto second
#[test]
fn drag_entry_on_other_entry() {
    setup();
    let fs_list: Vec<SubscriptionEntry> = dataset_simple_trio();
    let (fsc, _r_fsource) = prepare_subscription_move(fs_list);
    match fsc.drag_calc_positions(&vec![0], &vec![1, 0]) {
        Ok((from_entry, to_parent_id, to_folderpos)) => {
            panic!(
                "may not drag onto entry {:?} => P{} FP{}",
                from_entry, to_parent_id, to_folderpos
            );
        }
        Err(_e) => {}
    }
}

#[test]
fn check_paths_simple() {
    setup();
    let (stc, _fsource_r) = prepare_subscription_move(dataset_some_tree());
    assert_eq!(stc.get_by_path(&vec![0]).unwrap().subs_id, 1);
    assert_eq!(stc.get_by_path(&vec![0, 0]).unwrap().subs_id, 2);
    assert_eq!(stc.get_by_path(&vec![2]), None);
    assert_eq!(stc.get_by_path(&vec![0, 1]).unwrap().subs_id, 3);
    assert_eq!(stc.get_by_path(&vec![1]).unwrap().subs_id, 4);
}

// ------------------------------------

mod unzipper;

mod logger_config;
#[allow(unused_imports)]
#[macro_use]
extern crate log;
use std::sync::Once;

static TEST_SETUP: Once = Once::new();
fn setup() {
    TEST_SETUP.call_once(|| {
        let _r = logger_config::setup_fern_logger(logger_config::QuietFlags::Controller as u64);
        unzipper::unzip_some();
    });
}
