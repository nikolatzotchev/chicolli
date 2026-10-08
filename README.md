# Chicolli

Draw on top of your Wayland desktop. Chicolli opens a transparent overlay on every monitor (via [gtk4-layer-shell](https://github.com/wmww/gtk4-layer-shell)) with a pen, arrows, rectangles, a highlighter, text labels and an eraser.

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

Draw with the left mouse button, pick tools and colors from the toolbar or the keys below, and right click, Escape or the toolbar's close button to exit. Hover a toolbar button to see its key. Hold Shift to snap arrows to 45° steps and rectangles to squares.

| Key | Action |
|---|---|
| `1` `2` `3` | Pen, arrow, reversed arrow |
| `4` `5` `6` | Rectangle, text, highlighter |
| `e` | Eraser |
| `s` | Select: drag a shape to move it, click it to delete it |
| `r` `g` `b` `c` | Red, green, blue, color chooser |
| Scroll wheel | Line width (and eraser size) |
| `Ctrl+z` / `Ctrl+Shift+z` | Undo / redo |
| `Ctrl+x` | Clear (undo brings it back) |
| `Ctrl+c` / `Ctrl+s` | Copy the screen with the drawing / save it as a PNG |
| `Ctrl+Shift+c` / `Ctrl+Shift+s` | Drag out a region with `slurp`, then copy / save just that part |
| `d` | Pass-through: keep the drawing, use the desktop |
| `Escape` | Quit (while typing a label, it finishes the label first) |

With the text tool, click to place a label, type, Shift+Enter for a new line, Ctrl+V to paste, and Enter or Escape to finish. Click a label again to move it or keep typing.

The eraser rubs out whatever is under it, down to the desktop, and a circle around the pointer shows how much it takes. It is four times the line width, so scroll or use the toolbar's width buttons to resize it. Undo brings back what it erased, and anything drawn afterwards goes on top.

With the select tool, a dashed box shows which shape is under the pointer. Drag it to move it, or click it to delete it, without undoing what you drew after it. Undo takes back a move or a delete like any other step.

Copy and save take a screenshot of all monitors with your drawing on it, without the toolbar. Chicolli uses `grim` on wlroots compositors (Sway, Hyprland, Wayfire, ...) and falls back to the xdg-desktop-portal screenshot portal elsewhere. Saved pictures go to your pictures folder (`~/Pictures`) as `chicolli-<date>_<time>.png`. With `wl-copy` (from wl-clipboard) installed, a copied picture can still be pasted after Chicolli exits; without it, only while Chicolli runs.

Ctrl+Shift+C (or the toolbar's crop button) and Ctrl+Shift+S let you pick the part to take first: drag a box with the mouse (Escape cancels), and only that box is copied or saved, drawing included. Region capture needs both `slurp` and `grim`.

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
    "eraser": "e",
    "select": "s",
    "pass_through": "d",
    "red": "r",
    "green": "g",
    "blue": "b",
    "color_chooser": "c",
    "undo": "<Ctrl>z",
    "redo": "<Ctrl><Shift>z",
    "clear": "<Ctrl>x",
    "copy": "<Ctrl>c",
    "save": "<Ctrl>s",
    "copy_region": "<Ctrl><Shift>c",
    "save_region": "<Ctrl><Shift>s",
    "quit": "Escape"
  }
}
```

A key is a GTK key name (`"a"`, `"F1"`, `"space"`), optionally with `<Ctrl>`, `<Shift>`, `<Alt>` or `<Super>` in front, like `"<Ctrl><Shift>z"`. An empty string unbinds the action. `line_width` can be 1 to 200. Unknown keys and options, and files that are not valid JSON, are reported on stderr; a broken file keeps the previous settings. Config files from older versions (`draw_keybind`, `line_thickness`, ...) still work.

## License

[MIT](LICENSE)
