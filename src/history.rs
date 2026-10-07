use crate::geometry::Point;

/// An element that can be moved, so moving it can be undone and redone.
pub trait Movable {
    fn move_by(&mut self, by: Point);
}

/// Drawn elements with their undo and redo history.
///
/// Every element and every edit (Clear, delete, move) gets a step number when it happens.
/// Elements are kept in step order, so the latest step is either the last element or the
/// last edit, and Undo takes back whichever came later. Undone steps can be redone until
/// something new is drawn or edited.
pub struct History<T> {
    items: Vec<Entry<T>>,
    /// Edits other than drawing, oldest first.
    done: Vec<Edit<T>>,
    /// Undone steps, most recently undone last.
    undone: Vec<Undone<T>>,
    next_step: u64,
}

struct Entry<T> {
    /// Identifies the element for the moves made to it.
    id: u64,
    /// When the element was drawn, or last picked up to be edited.
    step: u64,
    item: T,
}

enum Edit<T> {
    /// What a Clear removed (empty once undone).
    Clear {
        step: u64,
        items: Vec<Entry<T>>,
    },
    Delete {
        step: u64,
        entry: Entry<T>,
    },
    Move {
        step: u64,
        id: u64,
        by: Point,
    },
}

impl<T> Edit<T> {
    fn step(&self) -> u64 {
        match self {
            Edit::Clear { step, .. } | Edit::Delete { step, .. } | Edit::Move { step, .. } => *step,
        }
    }
}

/// A step taken back by Undo, with what Redo needs to take it again.
enum Undone<T> {
    Item(Entry<T>),
    Clear { step: u64 },
    Delete { step: u64, id: u64 },
    Move { step: u64, id: u64, by: Point },
}

impl<T> Default for History<T> {
    fn default() -> Self {
        History {
            items: Vec::new(),
            done: Vec::new(),
            undone: Vec::new(),
            next_step: 0,
        }
    }
}

impl<T: Movable> History<T> {
    pub fn new() -> Self {
        Self::default()
    }

    fn step(&mut self) -> u64 {
        self.next_step += 1;
        self.next_step
    }

    /// The elements shown, oldest first.
    pub fn items(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator {
        self.items.iter().map(|e| &e.item)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index).map(|e| &mut e.item)
    }

    /// The newest element: the one being drawn or edited, if any is.
    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.items.last_mut().map(|e| &mut e.item)
    }

    /// Index of the newest element for which `f` returns true.
    pub fn rposition_mut(&mut self, mut f: impl FnMut(&mut T) -> bool) -> Option<usize> {
        self.items.iter_mut().rposition(|e| f(&mut e.item))
    }

    /// Adds an element being drawn. Call [`History::commit`] once it is kept, or
    /// [`History::pop`] to drop it.
    pub fn push(&mut self, item: T) {
        let step = self.step();
        self.items.push(Entry {
            id: step,
            step,
            item,
        });
    }

    /// Drops the newest element without recording anything, for elements that ended up
    /// drawing nothing.
    pub fn pop(&mut self) -> Option<T> {
        self.items.pop().map(|e| e.item)
    }

    /// Makes the element at `index` the newest, for editing it again. Undo then takes it
    /// back like a newly drawn element.
    pub fn raise(&mut self, index: usize) {
        if index < self.items.len() {
            let mut entry = self.items.remove(index);
            entry.step = self.step();
            self.items.push(entry);
        }
    }

    /// Records that something new was drawn or edited, which drops the redo steps.
    pub fn commit(&mut self) {
        self.undone.clear();
    }

    /// Removes every element, keeping them so Undo can bring them back. Returns whether
    /// there was anything to clear.
    pub fn clear(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }
        let step = self.step();
        let items = std::mem::take(&mut self.items);
        self.done.push(Edit::Clear { step, items });
        self.undone.clear();
        true
    }

    /// Deletes the element at `index`; Undo puts it back where it was.
    pub fn delete(&mut self, index: usize) -> bool {
        if index >= self.items.len() {
            return false;
        }
        let entry = self.items.remove(index);
        let step = self.step();
        self.done.push(Edit::Delete { step, entry });
        self.undone.clear();
        true
    }

    /// Records that the element at `index` was moved by `by` (it already was).
    pub fn moved(&mut self, index: usize, by: Point) {
        let Some(id) = self.items.get(index).map(|e| e.id) else {
            return;
        };
        let step = self.step();
        self.done.push(Edit::Move { step, id, by });
        self.undone.clear();
    }

    fn find(&mut self, id: u64) -> Option<usize> {
        self.items.iter().position(|e| e.id == id)
    }

    /// Takes back the latest element or edit. Returns whether there was one.
    pub fn undo(&mut self) -> bool {
        let item_step = self.items.last().map(|e| e.step);
        let edit_step = self.done.last().map(Edit::step);
        let undone = match (item_step, edit_step) {
            (None, None) => return false,
            (Some(item), edit) if edit.is_none_or(|edit| item > edit) => {
                Undone::Item(self.items.pop().expect("an element is shown"))
            }
            _ => match self.done.pop().expect("an edit was made") {
                Edit::Clear { step, items } => {
                    // Everything drawn after the Clear is undone already.
                    let newer = std::mem::replace(&mut self.items, items);
                    self.items.extend(newer);
                    Undone::Clear { step }
                }
                Edit::Delete { step, entry } => {
                    // Elements stay in step order, which puts it back where it was.
                    let at = self.items.partition_point(|e| e.step < entry.step);
                    let id = entry.id;
                    self.items.insert(at, entry);
                    Undone::Delete { step, id }
                }
                Edit::Move { step, id, by } => {
                    if let Some(i) = self.find(id) {
                        self.items[i].item.move_by(-by);
                    }
                    Undone::Move { step, id, by }
                }
            },
        };
        self.undone.push(undone);
        true
    }

    /// Re-applies the step undone last. Returns whether there was one.
    pub fn redo(&mut self) -> bool {
        match self.undone.pop() {
            Some(Undone::Item(entry)) => self.items.push(entry),
            Some(Undone::Clear { step }) => {
                if !self.items.is_empty() {
                    let items = std::mem::take(&mut self.items);
                    self.done.push(Edit::Clear { step, items });
                }
            }
            Some(Undone::Delete { step, id }) => {
                if let Some(i) = self.find(id) {
                    let entry = self.items.remove(i);
                    self.done.push(Edit::Delete { step, entry });
                }
            }
            Some(Undone::Move { step, id, by }) => {
                if let Some(i) = self.find(id) {
                    self.items[i].item.move_by(by);
                }
                self.done.push(Edit::Move { step, id, by });
            }
            None => return false,
        }
        true
    }

    /// Forgets every element, shown, cleared or undone, for which `keep` returns false.
    pub fn retain(&mut self, mut keep: impl FnMut(&T) -> bool) {
        self.items.retain(|e| keep(&e.item));
        self.done.retain_mut(|edit| match edit {
            Edit::Clear { items, .. } => {
                items.retain(|e| keep(&e.item));
                !items.is_empty()
            }
            Edit::Delete { entry, .. } => keep(&entry.item),
            Edit::Move { .. } => true,
        });
        self.undone.retain(|step| match step {
            Undone::Item(entry) => keep(&entry.item),
            _ => true,
        });
    }
}
