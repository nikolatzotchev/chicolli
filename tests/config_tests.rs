use chicolli::config::{parse_config, Configuration};

#[test]
fn missing_options_fall_back_to_defaults() {
    let (conf, unknown) = parse_config(r#"{ "keys": {} }"#).unwrap();
    assert_eq!(conf, Configuration::default());
    assert!(unknown.is_empty());

    let (conf, _) = parse_config(r#"{ "line_width": 9, "keys": { "arrow": "a" } }"#).unwrap();
    assert_eq!(conf.line_width, 9.0);
    assert_eq!(conf.keys.arrow, "a");
    assert_eq!(conf.keys.pen, "1");
    assert_eq!(conf.keys.undo, "<Ctrl>z");
}

#[test]
fn old_flat_option_names_still_work() {
    let content = r#"{
        "line_thickness": 3.0,
        "draw_keybind": "q",
        "disable_drawing": "p",
        "color_r": "R",
        "undo": "u",
        "clear_all": "<Alt>x"
    }"#;
    let (conf, unknown) = parse_config(content).unwrap();

    assert!(unknown.is_empty());
    assert_eq!(conf.line_width, 3.0);
    assert_eq!(conf.keys.pen, "q");
    assert_eq!(conf.keys.pass_through, "p");
    assert_eq!(conf.keys.red, "R");
    // The old undo and clear options implied Ctrl.
    assert_eq!(conf.keys.undo, "<Ctrl>u");
    assert_eq!(conf.keys.clear, "<Alt>x");
    assert_eq!(conf.keys.arrow, "2");
}

#[test]
fn new_names_win_over_old_ones() {
    let content =
        r#"{ "line_thickness": 3, "line_width": 4, "draw_keybind": "q", "keys": { "pen": "w" } }"#;
    let (conf, _) = parse_config(content).unwrap();
    assert_eq!(conf.line_width, 4.0);
    assert_eq!(conf.keys.pen, "w");
}

#[test]
fn unknown_options_are_reported() {
    let content = r#"{ "line_widht": 3, "keys": { "undo": "<Ctrl>u", "erase": "y" } }"#;
    let (_, unknown) = parse_config(content).unwrap();
    assert_eq!(
        unknown,
        vec!["line_widht".to_string(), "keys.erase".to_string()]
    );
}

#[test]
fn invalid_json_is_an_error() {
    assert!(parse_config("not json").is_err());
}
