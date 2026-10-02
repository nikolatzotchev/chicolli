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
