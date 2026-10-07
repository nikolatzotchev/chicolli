use chicolli::config::Configuration;
use chicolli::keybinds::{Binding, Keybinds};
use gtk::gdk::{Key, ModifierType};

#[test]
fn default_keybinds_resolve_to_gtk_keys() {
    let keys = Keybinds::from_config(&Configuration::default());

    let plain = |key| {
        Some(Binding {
            key,
            modifiers: ModifierType::empty(),
        })
    };
    assert_eq!(keys.pen, plain(Key::_1));
    assert_eq!(keys.highlighter, plain(Key::_6));
    assert_eq!(keys.eraser, plain(Key::e));
    assert_eq!(keys.select, plain(Key::s));
    assert_eq!(keys.pass_through, plain(Key::d));
    assert_eq!(
        keys.undo,
        Some(Binding {
            key: Key::z,
            modifiers: ModifierType::CONTROL_MASK,
        })
    );
    let ctrl = |key| {
        Some(Binding {
            key,
            modifiers: ModifierType::CONTROL_MASK,
        })
    };
    assert_eq!(keys.copy, ctrl(Key::c));
    assert_eq!(keys.save, ctrl(Key::s));
    assert_eq!(
        keys.quit,
        Some(Binding {
            key: Key::Escape,
            modifiers: ModifierType::empty(),
        })
    );
}

#[test]
fn unknown_or_empty_keys_bind_nothing() {
    let mut conf = Configuration::default();
    conf.keys.pen = "not-a-key".into();
    conf.keys.arrow = String::new();
    conf.keys.text = "<Hyper>t".into();
    conf.keys.rectangle = "F1".into();
    let keys = Keybinds::from_config(&conf);

    assert_eq!(keys.pen, None);
    assert_eq!(keys.arrow, None);
    assert_eq!(keys.text, None);
    assert_eq!(keys.rectangle.map(|b| b.key), Some(Key::F1));
}

#[test]
fn modifiers_parse_in_any_case_and_order() {
    let binding = Binding::parse("<ctrl><SHIFT>z").unwrap();
    assert_eq!(binding.key, Key::z);
    assert_eq!(
        binding.modifiers,
        ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK
    );
    assert_eq!(
        Binding::parse("<Super>space").unwrap().modifiers,
        ModifierType::SUPER_MASK
    );
    assert_eq!(Binding::parse("<Ctrl z"), None);
}

#[test]
fn bindings_need_their_exact_modifiers() {
    let undo = Binding::parse("<Ctrl>z").unwrap();
    assert!(undo.matches(Key::z, ModifierType::CONTROL_MASK));
    assert!(!undo.matches(Key::z, ModifierType::empty()));
    assert!(!undo.matches(Key::z, ModifierType::CONTROL_MASK | ModifierType::ALT_MASK));

    let pen = Binding::parse("1").unwrap();
    assert!(pen.matches(Key::_1, ModifierType::empty()));
    assert!(!pen.matches(Key::_1, ModifierType::CONTROL_MASK));

    // Shift is part of the key unless the binding names it.
    let upper = Binding::parse("R").unwrap();
    assert!(upper.matches(Key::R, ModifierType::SHIFT_MASK));
    let redo = Binding::parse("<Ctrl><Shift>z").unwrap();
    assert!(redo.matches(
        Key::Z,
        ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK
    ));
    assert!(!redo.matches(Key::z, ModifierType::CONTROL_MASK));
}

#[test]
fn redo_and_undo_do_not_overlap() {
    let keys = Keybinds::from_config(&Configuration::default());
    let (undo, redo) = (keys.undo.unwrap(), keys.redo.unwrap());
    let ctrl_shift = ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK;
    assert!(redo.matches(Key::Z, ctrl_shift));
    assert!(!undo.matches(Key::Z, ctrl_shift));
    assert!(!redo.matches(Key::z, ModifierType::CONTROL_MASK));
}
