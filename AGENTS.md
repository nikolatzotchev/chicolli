# Chicolli

GTK4 shell drawing tool for Wayland. Renders a transparent fullscreen overlay using gtk4-layer-shell and allows freehand drawing, arrows, rectangles, highlighter strokes, and text labels on top of the desktop.

## Commands

- **Build:** `cargo build` (requires native GTK 4.16+/layer-shell dependencies and a C linker on `PATH`; on older GTK use `cargo build --no-default-features`)
- **Release build:** `cargo build --release`
- **Run:** `cargo run`
- **Test:** `cargo test`
- **Check (typecheck):** `cargo check`
- **Lint:** `cargo clippy`
- **Nix build:** `nix-shell --run "cargo build"`
- **Nix dev shell:** `nix-shell`, then run normal Cargo commands inside the shell

On Nix/NixOS, use the checked-in `shell.nix`. It provides `rustc`, `cargo`, `stdenv.cc` (`cc` linker), `pkg-config`, GTK4, gtk4-layer-shell, Wayland, and related native libraries. If plain `cargo build` fails with `linker cc not found` or `pkg-config` errors, retry inside `nix-shell`.

## Dependencies

- Rust 2021 edition
- GTK4 (`gtk4` crate v0.10, feature `v4_10`; the default `hidpi-cursors` Cargo feature enables `v4_16`)
- `gio` v0.21
- `gtk4-layer-shell` v0.7.1 (Wayland layer shell integration)
- `serde` / `serde_json` (JSON config)
- `dirs` v6.0 (XDG config directory)
- `pangocairo` v0.21.5 (text rendering)
- System packages: `gtk4`, `gtk4-layer-shell`, `wayland` libs, `pkg-config`, and a C compiler/linker (`cc`, `gcc`, or `clang`)

## Architecture

```
src/
├── main.rs                 # App entry, GTK window setup, layer-shell config, input handling
├── config.rs               # JSON config read/write from ~/.config/chicolli/chicolli.json
├── colors.rs               # Color type alias (gtk::gdk::RGBA) and preset constants
├── cursors.rs              # Runtime Cairo-generated tool cursors (ToolCursor keeps them matched to tool and color)
├── toolbar.rs              # Overlay toolbar for tools, colors, line width, undo/clear
├── drawing.rs              # Module re-exports for drawing tools
├── drawing/
│   ├── drawing_tool.rs     # Point struct, DrawingTool trait, CurrentDrawingTool enum, snap helpers
│   ├── normal_line.rs      # Freehand line tool (B-spline interpolation)
│   ├── arrow.rs            # Arrow tool (line with arrowhead, reversible direction)
│   ├── normal_rectangle.rs # Rectangle tool
│   ├── highlighter.rs      # Semi-transparent freehand highlighter tool
│   └── text_label.rs       # Text label placement and drawing
└── styles/
    └── style.css           # Transparent window and toolbar CSS
cursors/                 # Custom cursor PNGs (pencil, arrow, rectangle)
shell.nix                # Nix development shell with native build dependencies
```

## Key Patterns

- **DrawingTool trait** (`src/drawing/drawing_tool.rs`): All tools implement `DrawingTool` with `press_mouse`, `release_mouse`, `motion_notify`, `draw`, `set_line_width`, `set_color`, `active`, `as_any_mut`, and optional `set_constrained` and `is_empty` (elements that would draw nothing are dropped on release so Undo never removes something invisible). Shared Cairo helpers (`set_source_color`, which honors the color's alpha, `stroke_smooth_path`, `stroke_relaxed_path`) live next to the trait.
- **State management**: Uses `Rc<RefCell<T>>` for shared mutable state across GTK closures.
- **Config**: JSON config at `~/.config/chicolli/chicolli.json` with keybinds and line thickness. Auto-created with defaults if missing. Partial configs merge with `Configuration::default()`.
- **Cursors**: Tool cursors are drawn at runtime in `src/cursors.rs` with Cairo on a 32-unit grid: dark ink, white halo, faint dark rim. Pen and highlighter show the current color; the arrows are a colored arrow from the hotspot (the plain arrow starts at a dot there, the reverse arrow's head tip sits on it); the rectangle pairs a crosshair with its toolbar glyph (`rectangle_icon_path` is shared with `toolbar.rs`). Each shape's working point sits exactly on its hotspot. Call `ToolCursor::show(tool, color)` wherever the tool or color changes. With `hidpi-cursors` (GTK 4.16+) they use `gdk::Cursor::from_callback` to render at the output scale and follow the theme cursor size; without it, a 32px `MemoryTexture`. The `cursors/` PNGs are old bundled assets the app no longer uses.
- **Toolbar**: `src/toolbar.rs` owns tool toggles, color presets/chooser swatch, line-width buttons, and undo/clear buttons. Keep toolbar state synchronized with keyboard shortcuts and mouse-wheel changes via `Toolbar::update`.
- **Constrained drawing**: `DrawingTool::set_constrained` is used for Shift-modified snapping. `snap_angle`, `snap_square`, the spline solver `spline_controls` and `arrow_head` live in `src/geometry.rs` (GTK-free, unit-tested in `tests/geometry_tests.rs`) and are re-exported where needed.
- **Layer behavior**: The main window uses layer-shell overlay mode and exclusive keyboard mode. The color chooser is a popover on the toolbar swatch (a popup of the overlay, so tiling compositors never tile it); the key handler passes keys through while it is open. Pass-through (`disable_drawing` key or the toolbar's pass-through toggle) shrinks the input region to the toolbar and sets `KeyboardMode::None` via `set_pass_through` in `main.rs`; a second launch re-activates the running instance (GApplication uniqueness) and restores drawing mode.

## Code Style

- No `use` wildcard imports except for GTK prelude (`use gtk::prelude::*`)
- GTK signal handlers use `glib::clone!` with explicit `#[strong]` / `#[weak]` captures for reference management
- `match` on key values for keybinding dispatch
- Drawing tools default to `colors::RED` initial color
- Keep Cargo dependency versions in `AGENTS.md`, `README.md`, and `Cargo.toml` aligned when updating dependencies
