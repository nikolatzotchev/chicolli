use chicolli::config::Configuration;

#[test]
fn merge_prefers_present_values_and_falls_back_to_defaults_source() {
    let primary = Configuration {
        line_thickness: Some(9.0),
        draw_keybind: None,
        arrow_keybind: Some("a".to_string()),
        reverse_arrow_keybind: None,
        rectangle_keybind: None,
        text_keybind: Some("t".to_string()),
        highlighter_keybind: None,
        disable_drawing: None,
        color_r: Some("R".to_string()),
        color_g: None,
        color_b: None,
        color_chooser: None,
        undo: Some("u".to_string()),
        clear_all: None,
    };

    let fallback = Configuration::default();
    let merged = primary.merge(fallback);

    assert_eq!(merged.line_thickness, Some(9.0));
    assert_eq!(merged.arrow_keybind, Some("a".to_string()));
    assert_eq!(merged.text_keybind, Some("t".to_string()));
    assert_eq!(merged.color_r, Some("R".to_string()));
    assert_eq!(merged.undo, Some("u".to_string()));
    assert_eq!(merged.draw_keybind, Some("1".to_string()));
    assert_eq!(merged.reverse_arrow_keybind, Some("3".to_string()));
    assert_eq!(merged.rectangle_keybind, Some("4".to_string()));
    assert_eq!(merged.highlighter_keybind, Some("6".to_string()));
    assert_eq!(merged.disable_drawing, Some("d".to_string()));
    assert_eq!(merged.color_g, Some("g".to_string()));
    assert_eq!(merged.color_b, Some("b".to_string()));
    assert_eq!(merged.color_chooser, Some("c".to_string()));
    assert_eq!(merged.clear_all, Some("x".to_string()));
}

#[test]
fn unknown_options_names_misspelled_keys_only() {
    let content = r#"{ "line_thickness": 3.0, "undo_keybind": "u", "draw_keybind": "q" }"#;
    assert_eq!(
        chicolli::config::unknown_options(content),
        vec!["undo_keybind".to_string()]
    );
    assert!(chicolli::config::unknown_options("not json").is_empty());
}
