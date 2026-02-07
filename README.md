# Chicolli

A GTK4 shell drawing tool for Wayland. Renders a transparent fullscreen overlay using [gtk4-layer-shell](https://github.com/wmww/gtk4-layer-shell) and allows freehand drawing, arrows, and rectangles on top of the desktop.

## Features

- Freehand drawing with B-spline interpolation
- Arrows (with reversible direction)
- Rectangles
- Quick color switching (red, green, blue) and a color chooser dialog
- Adjustable line thickness via scroll wheel
- Custom cursors per tool
- Configurable keybindings

## Dependencies

- GTK4
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

## Building

```sh
cargo build --release
```

## Installation

After building, copy the binary to a directory in your `$PATH`:

```sh
sudo cp target/release/gtk4-drawing-tool /usr/local/bin/chicolli
```

Or install it directly with Cargo:

```sh
cargo install --path .
```

This installs the binary as `gtk4-drawing-tool`. You can rename or symlink it:

```sh
ln -s ~/.cargo/bin/gtk4-drawing-tool ~/.cargo/bin/chicolli
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
| Exit | Right click |

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
  "disable_drawing": "d",
  "color_r": "r",
  "color_g": "g",
  "color_b": "b",
  "color_chooser": "c"
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
| `disable_drawing` | string | `"d"` | Key to dismiss the overlay (releases keyboard and input). |
| `color_r` | string | `"r"` | Key to switch color to red. |
| `color_g` | string | `"g"` | Key to switch color to green. |
| `color_b` | string | `"b"` | Key to switch color to blue. |
| `color_chooser` | string | `"c"` | Key to open the GTK color chooser dialog. |

Keybind values are GTK key names (e.g. `"1"`, `"a"`, `"F1"`, `"space"`).

### Custom cursors

Place PNG images in `~/.config/chicolli/cursors/` to use custom cursors for each tool:

| Filename | Tool |
|---|---|
| `pencil.png` | Freehand drawing |
| `arrow.png` | Arrow |
| `rectangle.png` | Rectangle |

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
