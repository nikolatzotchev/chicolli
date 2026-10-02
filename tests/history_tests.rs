use chicolli::history::History;

fn drawn(items: &[&'static str]) -> History<&'static str> {
    let mut history = History::new();
    for &item in items {
        history.items_mut().push(item);
        history.commit();
    }
    history
}

#[test]
fn undo_and_redo_walk_back_and_forth() {
    let mut history = drawn(&["a", "b"]);
    assert!(history.undo());
    assert_eq!(history.items(), ["a"]);
    assert!(history.redo());
    assert_eq!(history.items(), ["a", "b"]);
    assert!(!history.redo());
}

#[test]
fn clear_can_be_undone_and_redone() {
    let mut history = drawn(&["a", "b"]);
    assert!(history.clear());
    history.items_mut().push("c");
    history.commit();

    assert!(history.undo());
    assert!(history.items().is_empty());
    assert!(history.undo());
    assert_eq!(history.items(), ["a", "b"]);
    assert!(history.undo());
    assert_eq!(history.items(), ["a"]);

    assert!(history.redo());
    assert!(history.redo());
    assert!(history.items().is_empty());
    assert!(history.redo());
    assert_eq!(history.items(), ["c"]);
    assert!(!history.redo());
}

#[test]
fn clearing_nothing_is_not_a_step() {
    let mut history = drawn(&["a"]);
    history.undo();
    assert!(!history.clear());
    assert!(history.redo());
    assert_eq!(history.items(), ["a"]);
    assert!(history.undo());
    assert!(!history.undo());
}

#[test]
fn drawing_something_new_drops_redo() {
    let mut history = drawn(&["a", "b"]);
    history.undo();
    history.items_mut().push("c");
    history.commit();
    assert!(!history.redo());
    assert_eq!(history.items(), ["a", "c"]);

    history.undo();
    history.clear();
    assert!(!history.redo());
}

#[test]
fn retain_reaches_cleared_and_undone_elements() {
    let mut history = drawn(&["a1", "b1", "a2"]);
    history.clear();
    history.items_mut().push("b2");
    history.commit();
    history.undo();

    history.retain(|item| !item.starts_with('b'));
    assert!(!history.redo());
    assert!(history.undo());
    assert_eq!(history.items(), ["a1", "a2"]);
}
