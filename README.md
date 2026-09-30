# Chicolli

A GTK4 shell drawing tool for Wayland. Renders a transparent fullscreen overlay using [gtk4-layer-shell](https://github.com/wmww/gtk4-layer-shell) and allows freehand drawing, arrows, and rectangles on top of the desktop.

## Features

- Freehand drawing with B-spline interpolation
- Arrows (with reversible direction)
- Rectangles
- Text labels (click to place, type to annotate, Shift+Enter for a new line, Ctrl+V to paste; click a label again to move or extend it)
- Highlighter (semi-transparent freehand drawing in the selected color, four times the line width)
- Quick color switching (red, green, blue) and a color chooser popover
- Adjustable line thickness via scroll wheel
- Per-tool cursors that show the current color and stay sharp on HiDPI screens
- Configurable keybindings

## Dependencies

- GTK4 (4.16 or newer; see below for older versions)
- gtk4-layer-shell
- Wayland compositor
- pkg-config

### Fedora

```sh
sudo dnf install gtk4-devel gtk4-layer-shell-devel wayland-devel pkg-config
```

### Arch Linux

```sh
sudo pacman -S gtk4 gtk4-layer-shell wayland pkg-config
```

### Nix / NixOS

```sh
nix-shell --run "cargo build"
```

Or enter the shell first:

```sh
nix-shell
cargo build
```

## Building

```sh
cargo build --release
```

The default `hidpi-cursors` feature renders the tool cursors at your output's scale and needs GTK 4.16 or newer. On older GTK (for example Ubuntu 24.04, which ships 4.14), build without it; the cursors are then drawn at 1x and scaled by the compositor:

```sh
cargo build --release --no-default-features
```

## Installation

After building, copy the binary to a directory in your `$PATH`:

```sh
sudo cp target/release/chicolli /usr/local/bin/chicolli
```

Or install it directly with Cargo:

```sh
cargo install --path .
```

Optionally, copy the bundled cursors to the config directory:

```sh
mkdir -p ~/.config/chicolli/cursors
cp cursors/*.png ~/.config/chicolli/cursors/
```

## Usage

```sh
chicolli
```

| Action | Input |
|---|---|
| Draw | Left click and drag |
| Change line thickness | Scroll wheel |
| Use the desktop, keep the drawing | `d` or the pointer toggle at the end of the toolbar |
| Back to drawing | Click the pointer toggle (or any tool) again, or run `chicolli` again |
| Exit | Right click |

In pass-through mode the drawing stays on screen and clicks and keys go to the windows underneath; only the toolbar still takes clicks, so you can switch back from it. Launching `chicolli` again returns the running overlay to drawing mode with the drawing intact instead of opening a second one (this needs a D-Bus session bus, which desktop sessions provide).

## Configuration

Chicolli uses a JSON config file located at:

```
~/.config/chicolli/chicolli.json
```

On first run, the config file is created automatically with default values. You only need to specify the options you want to change — any missing fields fall back to their defaults.

### Default configuration

```json
{
  "line_thickness": 2.0,
  "draw_keybind": "1",
  "arrow_keybind": "2",
  "reverse_arrow_keybind": "3",
  "rectangle_keybind": "4",
  "text_keybind": "5",
  "highlighter_keybind": "6",
  "disable_drawing": "d",
  "color_r": "r",
  "color_g": "g",
  "color_b": "b",
  "color_chooser": "c",
  "undo": "z",
  "clear_all": "x"
}
```

### Options

| Option | Type | Default | Description |
|---|---|---|---|
| `line_thickness` | float | `2.0` | Initial stroke width in pixels. Can be adjusted at runtime with the scroll wheel. |
| `draw_keybind` | string | `"1"` | Key to switch to the freehand drawing tool. |
| `arrow_keybind` | string | `"2"` | Key to switch to the arrow tool (arrowhead at pointer end). |
| `reverse_arrow_keybind` | string | `"3"` | Key to switch to the reverse arrow tool (arrowhead at start). |
| `rectangle_keybind` | string | `"4"` | Key to switch to the rectangle tool. |
| `text_keybind` | string | `"5"` | Key to switch to the text label tool. Click to place, type to enter text (tool keys type normally while editing), Shift+Enter for a new line, Ctrl+V to paste, Enter or Escape to finish. Click an existing label with this tool to drag it or keep typing. The text size follows the line thickness. |
| `highlighter_keybind` | string | `"6"` | Key to switch to the highlighter tool. Draws semi-transparent strokes in the selected color, four times as wide as the line thickness. |
| `disable_drawing` | string | `"d"` | Key to switch to pass-through: the drawing stays visible while clicks and keys reach the desktop. Click the toolbar's pointer toggle or run `chicolli` again to resume drawing. |
| `color_r` | string | `"r"` | Key to switch color to red. |
| `color_g` | string | `"g"` | Key to switch color to green. |
| `color_b` | string | `"b"` | Key to switch color to blue. |
| `color_chooser` | string | `"c"` | Key to open the color chooser popover. |
| `undo` | string | `"z"` | Key (with Ctrl) to undo the last drawn element. |
| `clear_all` | string | `"x"` | Key (with Ctrl) to clear all drawn elements. |

Keybind values are GTK key names (e.g. `"1"`, `"a"`, `"F1"`, `"space"`).

### Custom cursors

Place PNG images in `~/.config/chicolli/cursors/` to use custom cursors for each tool:

| Filename | Tool |
|---|---|
| `pencil.png` | Freehand drawing |
| `arrow.png` | Arrow |
| `rectangle.png` | Rectangle |
| `text.png` | Text label |
| `highlighter.png` | Highlighter |

The images are scaled to 30×30 pixels. Default bundled cursors are included in the `cursors/` directory of the repository and can be copied to the config location.

## Compositor shortcut

Since chicolli is a Wayland overlay, you typically launch it with a keyboard shortcut in your compositor.

### Wayfire

Add the following to `~/.config/wayfire.ini` under the `[command]` section:

```ini
[command]
binding_chicolli = <super> KEY_D
command_chicolli = chicolli
```

Replace `<super> KEY_D` with your preferred key combination. Wayfire key names use the Linux input event codes (e.g. `KEY_A`, `KEY_F1`, `KEY_SPACE`).

### Sway

Add to `~/.config/sway/config`:

```
bindsym $mod+d exec chicolli
```

### Hyprland

Add to `~/.config/hypr/hyprland.conf`:

```
bind = SUPER, D, exec, chicolli
```

## License

[MIT](LICENSE)
