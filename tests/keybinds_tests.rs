use chicolli::config::Configuration;
use chicolli::keybinds::Keybinds;
use gtk::gdk::Key;

#[test]
fn default_keybinds_resolve_to_gtk_keys() {
    let keys = Keybinds::from_config(&Configuration::default());

    assert_eq!(keys.draw, Some(Key::_1));
    assert_eq!(keys.highlighter, Some(Key::_6));
    assert_eq!(keys.disable_drawing, Some(Key::d));
    assert_eq!(keys.undo, Some(Key::z));
    assert_eq!(keys.clear_all, Some(Key::x));
}

#[test]
fn unknown_or_missing_key_names_bind_nothing() {
    let conf = Configuration {
        draw_keybind: Some("not-a-key".to_string()),
        arrow_keybind: None,
        rectangle_keybind: Some("F1".to_string()),
        ..Configuration::default()
    };
    let keys = Keybinds::from_config(&conf);

    assert_eq!(keys.draw, None);
    assert_eq!(keys.arrow, None);
    assert_eq!(keys.rectangle, Some(Key::F1));
}
