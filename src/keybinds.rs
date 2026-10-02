use gtk::gdk::{Key, ModifierType};

use crate::config::Configuration;

/// Modifiers a binding must match exactly. Shift is left out because it already shows in
/// the key itself (`R` rather than `r`), unless a binding asks for `<Shift>` explicitly.
const EXACT_MODIFIERS: ModifierType = ModifierType::CONTROL_MASK
    .union(ModifierType::ALT_MASK)
    .union(ModifierType::SUPER_MASK);

/// One key with the modifiers that must be held for it, such as Ctrl+Z.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binding {
    pub key: Key,
    pub modifiers: ModifierType,
}

impl Binding {
    /// Parses a key name with optional modifiers in front: `"z"`, `"F1"`, `"<Ctrl>z"`,
    /// `"<Ctrl><Shift>z"`. Modifiers are Ctrl (or Control, Primary), Shift, Alt and
    /// Super, in any case. Returns `None` for anything else.
    pub fn parse(text: &str) -> Option<Self> {
        let mut modifiers = ModifierType::empty();
        let mut rest = text.trim();
        while let Some(after) = rest.strip_prefix('<') {
            let (name, tail) = after.split_once('>')?;
            modifiers |= match name.to_ascii_lowercase().as_str() {
                "ctrl" | "control" | "primary" => ModifierType::CONTROL_MASK,
                "shift" => ModifierType::SHIFT_MASK,
                "alt" => ModifierType::ALT_MASK,
                "super" => ModifierType::SUPER_MASK,
                _ => return None,
            };
            rest = tail;
        }
        Some(Binding {
            key: Key::from_name(rest)?,
            modifiers,
        })
    }

    /// How the binding reads in a tooltip, such as `Ctrl+Shift+Z`.
    pub fn label(&self) -> String {
        gtk::accelerator_get_label(self.key, self.modifiers).to_string()
    }

    /// Whether a key press with this modifier state triggers the binding.
    pub fn matches(&self, keyval: Key, state: ModifierType) -> bool {
        if state & EXACT_MODIFIERS != self.modifiers & EXACT_MODIFIERS {
            return false;
        }
        if self.modifiers.contains(ModifierType::SHIFT_MASK) {
            state.contains(ModifierType::SHIFT_MASK) && keyval.to_lower() == self.key.to_lower()
        } else {
            keyval == self.key
        }
    }
}

/// The configured keybinds, resolved from their names once per config load instead of
/// on every key press. An empty or unknown name binds nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keybinds {
    pub pen: Option<Binding>,
    pub arrow: Option<Binding>,
    pub reverse_arrow: Option<Binding>,
    pub rectangle: Option<Binding>,
    pub text: Option<Binding>,
    pub highlighter: Option<Binding>,
    pub pass_through: Option<Binding>,
    pub red: Option<Binding>,
    pub green: Option<Binding>,
    pub blue: Option<Binding>,
    pub color_chooser: Option<Binding>,
    pub undo: Option<Binding>,
    pub redo: Option<Binding>,
    pub clear: Option<Binding>,
    pub copy: Option<Binding>,
    pub save: Option<Binding>,
    pub quit: Option<Binding>,
}

/// Resolves one key, warning about names that do not parse.
fn parse(action: &str, text: &str) -> Option<Binding> {
    if text.trim().is_empty() {
        return None;
    }
    let binding = Binding::parse(text);
    if binding.is_none() {
        eprintln!("chicolli: unknown key {text:?} for keys.{action}, leaving it unbound");
    }
    binding
}

impl Keybinds {
    pub fn from_config(conf: &Configuration) -> Self {
        let k = &conf.keys;
        Keybinds {
            pen: parse("pen", &k.pen),
            arrow: parse("arrow", &k.arrow),
            reverse_arrow: parse("reverse_arrow", &k.reverse_arrow),
            rectangle: parse("rectangle", &k.rectangle),
            text: parse("text", &k.text),
            highlighter: parse("highlighter", &k.highlighter),
            pass_through: parse("pass_through", &k.pass_through),
            red: parse("red", &k.red),
            green: parse("green", &k.green),
            blue: parse("blue", &k.blue),
            color_chooser: parse("color_chooser", &k.color_chooser),
            undo: parse("undo", &k.undo),
            redo: parse("redo", &k.redo),
            clear: parse("clear", &k.clear),
            copy: parse("copy", &k.copy),
            save: parse("save", &k.save),
            quit: parse("quit", &k.quit),
        }
    }
}
