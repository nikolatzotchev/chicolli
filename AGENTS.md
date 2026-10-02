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
- GTK4 (`gtk4` crate v0.11, feature `v4_10`; the default `hidpi-cursors` Cargo feature enables `v4_16`)
- `gio` v0.22
- `gtk4-layer-shell` v0.8.1 (Wayland layer shell integration)
- `serde` / `serde_json` (JSON config)
- `dirs` v7.0 (XDG config directory)
- `pangocairo` v0.22.9 (text rendering)
- System packages: `gtk4`, `gtk4-layer-shell`, `wayland` libs, `pkg-config`, and a C compiler/linker (`cc`, `gcc`, or `clang`)

## Architecture

```
src/
├── main.rs                 # App entry, one layer-shell overlay per monitor, shared state, input handling
├── config.rs               # JSON config parsing (with old flat names) from ~/.config/chicolli/chicolli.json
├── keybinds.rs             # Keybinds resolved from config key names once per (re)load
├── colors.rs               # Color type alias (gtk::gdk::RGBA) and preset constants
├── capture.rs              # Screenshots of the annotated desktop (grim, else the xdg screenshot portal) for copy/save
├── cursors.rs              # Runtime Cairo-generated tool cursors (ToolCursor keeps them matched to tool and color)
├── toolbar.rs              # Overlay toolbar for tools, colors, line width, undo/redo/clear, pass-through, quit
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
shell.nix                # Nix development shell with native build dependencies
```

## Key Patterns

- **App ID**: `io.github.nikolatzotchev.Chicolli` (`APP_ID` in `main.rs`); GApplication uniqueness and notifications use it. `chicolli --version` prints the Cargo version.

- **DrawingTool trait** (`src/drawing/drawing_tool.rs`): All tools implement `DrawingTool` with `press_mouse`, `release_mouse`, `motion_notify`, `draw`, `set_line_width`, `set_color`, `active`, `as_any_mut`, and optional `set_constrained` and `is_empty` (elements that would draw nothing are dropped on release so Undo never removes something invisible). Shared Cairo helpers (`set_source_color`, which honors the color's alpha, `stroke_smooth_path`, `stroke_relaxed_path`) live next to the trait.
- **State management**: Uses `Rc<RefCell<T>>` for shared mutable state across GTK closures.
- **Config**: JSON config at `~/.config/chicolli/chicolli.json`: `line_width` (1 to 200, the same range the toolbar and scroll wheel keep to) plus a `keys` group (`pen`, `undo`, `quit`, ...; values are GTK key names with optional `<Ctrl>`/`<Shift>`/`<Alt>`/`<Super>` prefixes). Created as `{"keys": {}}` if missing; `config::parse_config` fills missing options from `Configuration::default()`, still accepts the old flat names (`draw_keybind`, `line_thickness`, `undo` implying Ctrl, ...) and reports unknown options. Keybinds are resolved to `keybinds::Binding`s (key plus exact Ctrl/Alt/Super modifiers) once per load (`src/keybinds.rs`); `State::watch_config` in `main.rs` reloads the file with a `gio::FileMonitor` when it is saved (in place or by rename), keeping the old config if the new one fails to parse.
- **Cursors**: Tool cursors are drawn at runtime in `src/cursors.rs` with Cairo on a 32-unit grid: dark ink, white halo, faint dark rim. Pen and highlighter show the current color; arrow and rectangle pair a crosshair with the toolbar glyph (`arrow_icon_path` / `rectangle_icon_path` are shared with `toolbar.rs`). Each shape's working point sits exactly on its hotspot. Call `ToolCursor::show(tool, color)` wherever the tool or color changes. With `hidpi-cursors` (GTK 4.16+) they use `gdk::Cursor::from_callback` (needs gdk4 0.11+; 0.10 freed the texture before GTK read it) to render at the output scale and follow the theme cursor size; without it, or at runtime on GTK 4.19.0 to 4.21.2 (whose Wayland backend shows callback cursors scale² too large, GTK issue 7888), a 32px `MemoryTexture`.
- **Toolbar**: `src/toolbar.rs` owns tool toggles, color presets/chooser swatch, line-width buttons, undo/redo/clear, pass-through and quit buttons. Keep toolbar state synchronized with keyboard shortcuts and mouse-wheel changes via `Toolbar::update`; `Toolbar::show_keybinds` puts each button's key in its tooltip and is called again on config reload.
- **Constrained drawing**: `DrawingTool::set_constrained` is used for Shift-modified snapping. `snap_angle`, `snap_square`, the spline solver `spline_controls` and `arrow_head` live in `src/geometry.rs` (GTK-free, unit-tested in `tests/geometry_tests.rs`) and are re-exported where needed.
- **Layer behavior**: Every monitor gets its own fullscreen overlay window (a `Canvas` in `main.rs`), kept in step with monitor hotplug by `State::sync_monitors`; the windows share one `State` (tool, color, width, elements tagged with the canvas they were started on, so Undo removes the latest element on any monitor). Element points are in global layout coordinates (each overlay adds its monitor's `geometry()` origin to input and translates by it when drawing), so a stroke dragged past a monitor edge continues on the neighbouring monitor whatever their resolutions or scales. There is one toolbar, and it moves to the overlay the pointer enters. The windows use layer-shell overlay mode and exclusive keyboard mode. The color chooser is a popover on the toolbar swatch (a popup of the overlay, so tiling compositors never tile it); the key handler passes keys through while it is open. Pass-through (`disable_drawing` key or the toolbar's pass-through toggle) shrinks the input region to the toolbar and sets `KeyboardMode::None` via `State::set_pass_through` in `main.rs` (overlays without the toolbar take no input); a second launch re-activates the running instance (GApplication uniqueness) and restores drawing mode.
- **Copy/save**: the `copy`/`save` keys (Ctrl+C/Ctrl+S) call `State::capture`, which ends text input, hides the toolbar, waits 200ms for the compositor, and screenshots all outputs via `src/capture.rs` (`grim -`, else the `org.freedesktop.portal.Screenshot` portal, non-interactive). Copy goes through `wl-copy` so it outlives the app, falling back to the GDK clipboard; save writes `chicolli-<stamp>.png` to the XDG pictures dir (else `~/Pictures`). The outcome is reported on stderr and as a `gio::Notification`.

## Code Style

- No `use` wildcard imports except for GTK prelude (`use gtk::prelude::*`)
- GTK signal handlers use `glib::clone!` with explicit `#[strong]` / `#[weak]` captures for reference management
- `match` on key values for keybinding dispatch
- Drawing tools default to `colors::RED` initial color
- Keep Cargo dependency versions in `AGENTS.md`, `README.md`, and `Cargo.toml` aligned when updating dependencies
