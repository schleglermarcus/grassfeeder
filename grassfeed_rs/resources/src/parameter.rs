pub const DOWNLOADER_MAX_NUM_THREADS: usize = 10;

pub const ICON_SIZE_LIMIT_BYTES: usize = 10000;

/// Searching for unread-state not present:  on each scan attempt, how many unread-jobs do we create.
pub const SCAN_EMPTY_UNREAD_GROUP: u8 = 5;

/// On each scheduled check, how many download jobs do we create
pub const FETCH_PROCESS_ONETIME_LIMIT: usize = 2;

/// On each subsciption update time check, how many time compares we do at one time.
pub const CHECK_MESSAGE_COUNTS_SET_SIZE: usize = 5;
