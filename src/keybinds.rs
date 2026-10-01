use gtk::gdk::Key;

use crate::config::Configuration;

/// The configured keybinds, resolved from their GTK key names once per config load
/// instead of on every key press. A missing or unknown key name binds nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keybinds {
    pub draw: Option<Key>,
    pub arrow: Option<Key>,
    pub reverse_arrow: Option<Key>,
    pub rectangle: Option<Key>,
    pub text: Option<Key>,
    pub highlighter: Option<Key>,
    pub disable_drawing: Option<Key>,
    pub color_r: Option<Key>,
    pub color_g: Option<Key>,
    pub color_b: Option<Key>,
    pub color_chooser: Option<Key>,
    pub undo: Option<Key>,
    pub clear_all: Option<Key>,
}

/// Resolves one key name, warning about names GTK does not know.
fn parse(option: &str, name: Option<&str>) -> Option<Key> {
    let name = name?;
    let key = Key::from_name(name);
    if key.is_none() {
        eprintln!("chicolli: unknown key name {name:?} for {option}, leaving it unbound");
    }
    key
}

impl Keybinds {
    pub fn from_config(conf: &Configuration) -> Self {
        Keybinds {
            draw: parse("draw_keybind", conf.draw_keybind.as_deref()),
            arrow: parse("arrow_keybind", conf.arrow_keybind.as_deref()),
            reverse_arrow: parse(
                "reverse_arrow_keybind",
                conf.reverse_arrow_keybind.as_deref(),
            ),
            rectangle: parse("rectangle_keybind", conf.rectangle_keybind.as_deref()),
            text: parse("text_keybind", conf.text_keybind.as_deref()),
            highlighter: parse("highlighter_keybind", conf.highlighter_keybind.as_deref()),
            disable_drawing: parse("disable_drawing", conf.disable_drawing.as_deref()),
            color_r: parse("color_r", conf.color_r.as_deref()),
            color_g: parse("color_g", conf.color_g.as_deref()),
            color_b: parse("color_b", conf.color_b.as_deref()),
            color_chooser: parse("color_chooser", conf.color_chooser.as_deref()),
            undo: parse("undo", conf.undo.as_deref()),
            clear_all: parse("clear_all", conf.clear_all.as_deref()),
        }
    }
}
