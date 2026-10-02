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

On GTK 4.20 (4.19.0 to 4.21.2) Chicolli also uses the 1x cursors, because those versions draw scaled cursors far too large on scaled monitors. That keeps the cursor the same size on every monitor, though it looks a bit soft on HiDPI screens. Once you update to GTK 4.22 or newer, the sharp scaled cursors come back on their own, with no rebuild needed.

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
| `Ctrl+z` / `Ctrl+Shift+z` | Undo / redo |
| `Ctrl+x` | Clear (undo brings it back) |
| `Ctrl+c` / `Ctrl+s` | Copy the screen with the drawing / save it as a PNG |
| `d` | Pass-through: keep the drawing, use the desktop |

With the text tool, click to place a label, type, Shift+Enter for a new line, Ctrl+V to paste, and Enter or Escape to finish. Click a label again to move it or keep typing.

Copy and save take a screenshot of all monitors with your drawing on it, without the toolbar. Chicolli uses `grim` on wlroots compositors (Sway, Hyprland, Wayfire, ...) and falls back to the xdg-desktop-portal screenshot portal elsewhere. Saved pictures go to your pictures folder (`~/Pictures`) as `chicolli-<date>_<time>.png`. With `wl-copy` (from wl-clipboard) installed, a copied picture can still be pasted after Chicolli exits; without it, only while Chicolli runs.

In pass-through mode only the toolbar takes clicks. Click its pointer toggle (or any tool), or run `chicolli` again, to get back to drawing with everything still there.

## Configure

Settings live in `~/.config/chicolli/chicolli.json`, created empty on first run. Add only what you want to change; everything else keeps its default, and changes apply as soon as you save. All options with their defaults:

```json
{
  "line_width": 5.0,
  "keys": {
    "pen": "1",
    "arrow": "2",
    "reverse_arrow": "3",
    "rectangle": "4",
    "text": "5",
    "highlighter": "6",
    "pass_through": "d",
    "red": "r",
    "green": "g",
    "blue": "b",
    "color_chooser": "c",
    "undo": "<Ctrl>z",
    "redo": "<Ctrl><Shift>z",
    "clear": "<Ctrl>x",
    "copy": "<Ctrl>c",
    "save": "<Ctrl>s"
  }
}
```

A key is a GTK key name (`"a"`, `"F1"`, `"space"`), optionally with `<Ctrl>`, `<Shift>`, `<Alt>` or `<Super>` in front, like `"<Ctrl><Shift>z"`. An empty string unbinds the action. Unknown keys and options, and files that are not valid JSON, are reported on stderr; a broken file keeps the previous settings. Config files from older versions (`draw_keybind`, `line_thickness`, ...) still work.

## License

[MIT](LICENSE)
