# Changelog

All notable changes to Chicolli are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed

- The overlay now covers panels such as waybar instead of stopping at their edge, so
  you can draw over them and strokes line up across monitors with different panels.
- The overlays use the layer namespace `chicolli`, so compositor rules such as
  Hyprland's `layerrule = noanim, chicolli` can match them.

## [1.0.0] - 2026-10-10

The first stable release.

### Drawing

- Pen, arrow, reversed arrow, rectangle, highlighter and text label tools, with Shift
  snapping arrows to 45° steps and rectangles to squares.
- Text labels take multiple lines (Shift+Enter) and pasted text (Ctrl+V), and can be
  clicked again to move them or keep typing.
- An eraser that rubs out what is under it down to the desktop, with an outline of its size.
- A select tool that moves a shape by dragging it or deletes it with a click.
- Undo and redo for every step, including Clear, moves and deletes.
- A color chooser popover, red, green and blue presets, and line width from 1 to 200 with
  the scroll wheel or the toolbar.

### Desktop

- An overlay on every monitor, kept in step with hotplugged monitors; strokes continue
  across monitors of any resolution or scale.
- A toolbar with drawn icons that follows the pointer to the active monitor, and shows
  each button's key in its tooltip.
- Tool cursors drawn at the output's scale on GTK 4.16 and newer.
- Pass-through mode that keeps the drawing while the desktop takes input; running
  `chicolli` again returns to drawing.
- Copy (Ctrl+C) or save (Ctrl+S) the annotated screen with `grim` or the screenshot
  portal, or just a region picked with `slurp` (Ctrl+Shift+C / Ctrl+Shift+S or the toolbar's
  crop button).
- A desktop file and icon under the app ID `io.github.nikolatzotchev.Chicolli`.

### Configuration

- `~/.config/chicolli/chicolli.json` sets the line width and every key binding, and is
  reloaded as soon as it is saved. Config files with the older flat option names still work.
- `chicolli --version` prints the version.

### Known limitations

- Text labels take one character per key, so dead keys and input methods (for example
  ´ + e, or CJK input) do not compose yet.

[1.0.0]: https://github.com/nikolatzotchev/chicolli/releases/tag/v1.0.0
