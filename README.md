# Chicolli

Draw on top of your Wayland desktop. Chicolli opens a transparent overlay on every monitor (via [gtk4-layer-shell](https://github.com/wmww/gtk4-layer-shell)) with a pen, arrows, rectangles, a highlighter and text labels.

## Install

You need GTK 4, gtk4-layer-shell, Wayland and pkg-config:

```sh
sudo dnf install gtk4-devel gtk4-layer-shell-devel wayland-devel pkg-config  # Fedora
sudo pacman -S gtk4 gtk4-layer-shell wayland pkg-config                      # Arch
nix-shell                                                                    # Nix: then build inside the shell
```

Then build and install:

```sh
cargo install --path .
```

On GTK older than 4.16 (e.g. Ubuntu 24.04) add `--no-default-features`; the cursors are then drawn at 1x instead of your screen's scale.

Bind `chicolli` to a key in your compositor, for example:

- Sway: `bindsym $mod+d exec chicolli`
- Hyprland: `bind = SUPER, D, exec, chicolli`
- Wayfire, under `[command]`: `binding_chicolli = <super> KEY_D` and `command_chicolli = chicolli`

## Use

Draw with the left mouse button, pick tools and colors from the toolbar or the keys below, and right click to exit. Hold Shift to snap arrows to 45° steps and rectangles to squares.

| Key | Action |
|---|---|
| `1` `2` `3` | Pen, arrow, reversed arrow |
| `4` `5` `6` | Rectangle, text, highlighter |
| `r` `g` `b` `c` | Red, green, blue, color chooser |
| Scroll wheel | Line width |
| `Ctrl+z` / `Ctrl+x` | Undo / clear |
| `d` | Pass-through: keep the drawing, use the desktop |

With the text tool, click to place a label, type, Shift+Enter for a new line, Ctrl+V to paste, and Enter or Escape to finish. Click a label again to move it or keep typing.

In pass-through mode only the toolbar takes clicks. Click its pointer toggle (or any tool), or run `chicolli` again, to get back to drawing with everything still there.

## Configure

Settings live in `~/.config/chicolli/chicolli.json`, created on first run. Every option is optional, and changes apply as soon as you save:

```json
{
  "line_thickness": 5.0,
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

Keys are GTK key names such as `"a"`, `"F1"` or `"space"`; `undo` and `clear_all` are pressed with Ctrl. Unknown key names, unknown options and files that are not valid JSON are reported on stderr, and a broken file keeps the previous settings.

## License

[MIT](LICENSE)
