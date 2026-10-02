/// Drawn elements with their undo and redo history.
///
/// Undo takes back the latest element, or a Clear. Because Clear empties the list, every
/// element still shown is newer than the last Clear, so Undo pops elements first and
/// only then brings back what the last Clear removed. Undone steps can be redone until
/// something new is drawn.
pub struct History<T> {
    items: Vec<T>,
    /// What each Clear removed, oldest first.
    cleared: Vec<Vec<T>>,
    /// Undone steps, most recently undone last.
    undone: Vec<Undone<T>>,
}

enum Undone<T> {
    Item(T),
    Clear,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        History {
            items: Vec::new(),
            cleared: Vec::new(),
            undone: Vec::new(),
        }
    }
}

impl<T> History<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// The elements shown, oldest first.
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// The elements shown, for adding and editing the element being drawn. Call
    /// [`History::commit`] once a new or edited element is kept.
    pub fn items_mut(&mut self) -> &mut Vec<T> {
        &mut self.items
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
        self.cleared.push(std::mem::take(&mut self.items));
        self.undone.clear();
        true
    }

    /// Takes back the latest element or Clear. Returns whether there was one.
    pub fn undo(&mut self) -> bool {
        if let Some(item) = self.items.pop() {
            self.undone.push(Undone::Item(item));
        } else if let Some(items) = self.cleared.pop() {
            self.items = items;
            self.undone.push(Undone::Clear);
        } else {
            return false;
        }
        true
    }

    /// Re-applies the step undone last. Returns whether there was one.
    pub fn redo(&mut self) -> bool {
        match self.undone.pop() {
            Some(Undone::Item(item)) => self.items.push(item),
            Some(Undone::Clear) => {
                if !self.items.is_empty() {
                    self.cleared.push(std::mem::take(&mut self.items));
                }
            }
            None => return false,
        }
        true
    }

    /// Forgets every element, shown, cleared or undone, for which `keep` returns false.
    pub fn retain(&mut self, mut keep: impl FnMut(&T) -> bool) {
        self.items.retain(&mut keep);
        for items in &mut self.cleared {
            items.retain(&mut keep);
        }
        self.cleared.retain(|items| !items.is_empty());
        self.undone.retain(|step| match step {
            Undone::Item(item) => keep(item),
            Undone::Clear => true,
        });
    }
}
