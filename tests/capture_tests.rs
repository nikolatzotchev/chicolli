use chicolli::capture::save_path;

#[test]
fn save_path_skips_names_that_are_taken() {
    let dir = std::env::temp_dir().join(format!("chicolli-capture-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let stamp = "2026-10-02_09-30-00";

    let first = save_path(&dir, stamp);
    assert_eq!(first, dir.join("chicolli-2026-10-02_09-30-00.png"));
    std::fs::write(&first, b"png").unwrap();
    let second = save_path(&dir, stamp);
    assert_eq!(second, dir.join("chicolli-2026-10-02_09-30-00-2.png"));
    std::fs::write(&second, b"png").unwrap();
    assert_eq!(
        save_path(&dir, stamp),
        dir.join("chicolli-2026-10-02_09-30-00-3.png")
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn slurp_outcome_tells_cancel_from_failure() {
    use chicolli::capture::slurp_outcome;
    assert_eq!(
        slurp_outcome(true, "10,20 300x200\n", ""),
        Ok(Some("10,20 300x200".to_owned()))
    );
    assert_eq!(slurp_outcome(true, "\n", ""), Ok(None));
    assert_eq!(slurp_outcome(false, "", "selection cancelled\n"), Ok(None));
    assert_eq!(
        slurp_outcome(false, "", "failed to create display\n"),
        Err("slurp failed (failed to create display)".to_owned())
    );
}
