use chicolli::geometry::Point;
use chicolli::history::{History, Movable};

/// A named element with a position, so moves can be checked.
#[derive(Debug, Clone, Copy)]
struct Item {
    name: &'static str,
    at: (f64, f64),
}

impl Movable for Item {
    fn move_by(&mut self, by: Point) {
        self.at = (self.at.0 + by.0, self.at.1 + by.1);
    }
}

fn item(name: &'static str) -> Item {
    Item {
        name,
        at: (0.0, 0.0),
    }
}

fn draw(history: &mut History<Item>, name: &'static str) {
    history.push(item(name));
    history.commit();
}

fn drawn(names: &[&'static str]) -> History<Item> {
    let mut history = History::new();
    for &name in names {
        draw(&mut history, name);
    }
    history
}

fn names(history: &History<Item>) -> Vec<&'static str> {
    history.items().map(|item| item.name).collect()
}

fn position(history: &History<Item>, name: &str) -> (f64, f64) {
    history.items().find(|item| item.name == name).unwrap().at
}

#[test]
fn undo_and_redo_walk_back_and_forth() {
    let mut history = drawn(&["a", "b"]);
    assert!(history.undo());
    assert_eq!(names(&history), ["a"]);
    assert!(history.redo());
    assert_eq!(names(&history), ["a", "b"]);
    assert!(!history.redo());
}

#[test]
fn clear_can_be_undone_and_redone() {
    let mut history = drawn(&["a", "b"]);
    assert!(history.clear());
    draw(&mut history, "c");

    assert!(history.undo());
    assert!(history.is_empty());
    assert!(history.undo());
    assert_eq!(names(&history), ["a", "b"]);
    assert!(history.undo());
    assert_eq!(names(&history), ["a"]);

    assert!(history.redo());
    assert!(history.redo());
    assert!(history.is_empty());
    assert!(history.redo());
    assert_eq!(names(&history), ["c"]);
    assert!(!history.redo());
}

#[test]
fn clearing_nothing_is_not_a_step() {
    let mut history = drawn(&["a"]);
    history.undo();
    assert!(!history.clear());
    assert!(history.redo());
    assert_eq!(names(&history), ["a"]);
    assert!(history.undo());
    assert!(!history.undo());
}

#[test]
fn drawing_something_new_drops_redo() {
    let mut history = drawn(&["a", "b"]);
    history.undo();
    draw(&mut history, "c");
    assert!(!history.redo());
    assert_eq!(names(&history), ["a", "c"]);

    history.undo();
    history.clear();
    assert!(!history.redo());
}

#[test]
fn retain_reaches_cleared_and_undone_elements() {
    let mut history = drawn(&["a1", "b1", "a2"]);
    history.clear();
    draw(&mut history, "b2");
    history.undo();

    history.retain(|item| !item.name.starts_with('b'));
    assert!(!history.redo());
    assert!(history.undo());
    assert_eq!(names(&history), ["a1", "a2"]);
}

#[test]
fn delete_takes_out_one_element_and_undo_puts_it_back_in_place() {
    let mut history = drawn(&["a", "b", "c"]);
    assert!(history.delete(0));
    assert_eq!(names(&history), ["b", "c"]);

    assert!(history.undo());
    assert_eq!(names(&history), ["a", "b", "c"]);
    assert!(history.redo());
    assert_eq!(names(&history), ["b", "c"]);

    // Deleting is the latest step, so Undo takes it back before "c".
    assert!(history.undo());
    assert!(history.undo());
    assert_eq!(names(&history), ["a", "b"]);
}

#[test]
fn moves_are_undone_and_redone_in_order() {
    let mut history = drawn(&["a", "b"]);
    history.get_mut(0).unwrap().move_by(Point(10.0, 5.0));
    history.moved(0, Point(10.0, 5.0));
    draw(&mut history, "c");

    assert!(history.undo());
    assert_eq!(names(&history), ["a", "b"]);
    assert_eq!(position(&history, "a"), (10.0, 5.0));
    assert!(history.undo());
    assert_eq!(position(&history, "a"), (0.0, 0.0));
    assert_eq!(names(&history), ["a", "b"]);

    assert!(history.redo());
    assert_eq!(position(&history, "a"), (10.0, 5.0));
    assert!(history.redo());
    assert_eq!(names(&history), ["a", "b", "c"]);
}

#[test]
fn deleting_a_moved_element_undoes_back_to_where_it_started() {
    let mut history = drawn(&["a", "b"]);
    history.get_mut(1).unwrap().move_by(Point(3.0, 4.0));
    history.moved(1, Point(3.0, 4.0));
    history.delete(1);

    assert!(history.undo());
    assert_eq!(position(&history, "b"), (3.0, 4.0));
    assert!(history.undo());
    assert_eq!(position(&history, "b"), (0.0, 0.0));
    assert!(history.undo());
    assert_eq!(names(&history), ["a"]);
}

#[test]
fn editing_drops_redo() {
    let mut history = drawn(&["a", "b"]);
    history.undo();
    history.delete(0);
    assert!(!history.redo());

    let mut history = drawn(&["a", "b"]);
    history.undo();
    history.moved(0, Point(1.0, 1.0));
    assert!(!history.redo());
}

#[test]
fn a_raised_element_is_undone_first() {
    let mut history = drawn(&["a", "b", "c"]);
    history.raise(0);
    history.commit();
    assert_eq!(names(&history), ["b", "c", "a"]);
    assert!(history.undo());
    assert_eq!(names(&history), ["b", "c"]);
}

#[test]
fn undoing_a_delete_after_clear_keeps_the_order() {
    let mut history = drawn(&["a", "b"]);
    history.delete(1);
    history.clear();
    draw(&mut history, "c");
    history.undo();
    history.undo();
    assert_eq!(names(&history), ["a"]);
    history.undo();
    assert_eq!(names(&history), ["a", "b"]);
}

#[test]
fn retain_drops_deleted_elements_too() {
    let mut history = drawn(&["a", "b1"]);
    history.delete(1);
    history.retain(|item| !item.name.starts_with('b'));
    // Nothing is left to bring back, so Undo goes straight to "a".
    assert!(history.undo());
    assert!(history.is_empty());
}
