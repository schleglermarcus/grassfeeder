use std::collections::HashSet;
use ui_gtk::IntCommands;

#[test]
pub fn int_commands_test() {
    // setup();
    let mut rset: HashSet<IntCommands> = HashSet::new();
    rset.insert(IntCommands::UpdateTreeModel(0));
    rset.insert(IntCommands::UpdateTreeModelSingle(0, vec![0]));
    rset.insert(IntCommands::UpdateTreeModel(0));
    let mut list = rset.iter().collect::<Vec<_>>();
    list.sort();
    let mut i = list.iter();
    assert_eq!(i.next(), Some(&&IntCommands::UpdateTreeModel(0)));
    assert_eq!(
        i.next(),
        Some(&&IntCommands::UpdateTreeModelSingle(0, vec![0]))
    );
    assert_eq!(i.next(), None);
}
