# Chicolli

GTK4 shell drawing tool for Wayland. Renders a transparent fullscreen overlay using gtk4-layer-shell and allows freehand drawing, arrows, and rectangles on top of the desktop.

## Commands

- **Build:** `cargo build`
- **Release build:** `cargo build --release`
- **Run:** `cargo run`
- **Test:** `cargo test`
- **Check (typecheck):** `cargo check`
- **Lint:** `cargo clippy`

## Dependencies

- Rust 2021 edition
- GTK4 (`gtk4` crate v0.7.1, feature `v4_10`)
- `gtk4-layer-shell` (Wayland layer shell integration)
- `serde` / `serde_json` (JSON config)
- `dirs` (XDG config directory)
- System packages: `gtk4`, `gtk4-layer-shell`, `wayland` libs, `pkg-config`

## Architecture

```
src/
├── main.rs              # App entry, GTK window setup, layer-shell config, input handling
├── config.rs            # JSON config read/write from ~/.config/chicolli/chicolli.json
├── colors.rs            # Color type alias (gtk::gdk::RGBA) and preset constants
├── drawing.rs           # Module re-exports for drawing tools
├── drawing/
│   ├── drawing_tool.rs  # Point struct, DrawingTool trait, CurrentDrawingTool enum
│   ├── normal_line.rs   # Freehand line tool (B-spline interpolation)
│   ├── arrow.rs         # Arrow tool (line with arrowhead, reversible direction)
│   └── normal_rectangle.rs  # Rectangle tool
└── styles/
    └── style.css        # Transparent window background
cursors/                 # Custom cursor PNGs (pencil, arrow, rectangle)
```

## Key Patterns

- **DrawingTool trait** (`src/drawing/drawing_tool.rs`): All tools implement `DrawingTool` with `press_mouse`, `release_mouse`, `motion_notify`, `draw`, `set_line_width`, `set_color`, `active`.
- **State management**: Uses `Rc<RefCell<T>>` for shared mutable state across GTK closures.
- **Config**: JSON config at `~/.config/chicolli/chicolli.json` with keybinds and line thickness. Auto-created with defaults if missing. Partial configs merge with `Configuration::minimal()`.
- **Custom cursors**: Loaded from `~/.config/chicolli/cursors/` (pencil.png, arrow.png, rectangle.png).

## Code Style

- No `use` wildcard imports except for GTK prelude (`use gtk::prelude::*`)
- GTK signal handlers use `glib::clone!(@strong/@weak ...)` for reference management
- `match` on key values for keybinding dispatch
- Drawing tools default to `colors::RED` initial color
