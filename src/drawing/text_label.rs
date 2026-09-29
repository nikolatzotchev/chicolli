use std::any::Any;
use std::cell::Cell;

use gtk::cairo::Context;
use gtk::pango;

use crate::colors;

use super::drawing_tool::{report, set_source_color, DrawingTool, Point};

/// Font size in points is `FONT_SIZE_BASE + line_width * FONT_SIZE_PER_WIDTH`, so each
/// scroll step changes the size noticeably without jumping straight to huge text.
const FONT_SIZE_BASE: f64 = 12.0;
const FONT_SIZE_PER_WIDTH: f64 = 3.0;
/// Extra margin around a label that still counts as clicking on it.
const HIT_MARGIN: f64 = 6.0;

/// Screen-space box of a drawn label: x, y, width, height.
type Bounds = (f64, f64, f64, f64);

pub struct TextLabel {
    /// Left edge of the text, vertically centered on the first line, so the text appears
    /// right where the I-beam cursor was clicked.
    position: Option<Point>,
    text: String,
    editing: bool,
    /// Offset from the grab point to `position` while the label is being dragged.
    drag_offset: Option<Point>,
    line_width: f64,
    color: colors::Color,
    /// Where the label was last drawn, used to click on it again.
    bounds: Cell<Option<Bounds>>,
}

impl Default for TextLabel {
    fn default() -> Self {
        Self::new()
    }
}

impl TextLabel {
    pub fn new() -> TextLabel {
        TextLabel {
            position: None,
            text: String::new(),
            editing: false,
            drag_offset: None,
            line_width: 5.0,
            color: colors::RED,
            bounds: Cell::new(None),
        }
    }

    pub fn push_char(&mut self, c: char) {
        self.text.push(c);
    }

    /// Appends pasted text, dropping control characters other than line breaks.
    pub fn push_str(&mut self, s: &str) {
        let s = s.replace("\r\n", "\n");
        self.text
            .extend(s.chars().filter(|c| *c == '\n' || !c.is_control()));
    }

    pub fn pop_char(&mut self) {
        self.text.pop();
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Ends editing and leaves the label where it is.
    pub fn commit(&mut self) {
        self.editing = false;
        self.drag_offset = None;
    }

    /// Re-opens a finished label for typing and starts dragging it from `grab`.
    pub fn edit_and_grab(&mut self, grab: Point) {
        self.editing = true;
        if let Some(pos) = self.position {
            self.drag_offset = Some(pos - grab);
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }

    /// Whether `point` is on the label as it was last drawn.
    pub fn contains(&self, point: Point) -> bool {
        match self.bounds.get() {
            Some((x, y, w, h)) => {
                point.0 >= x - HIT_MARGIN
                    && point.0 <= x + w + HIT_MARGIN
                    && point.1 >= y - HIT_MARGIN
                    && point.1 <= y + h + HIT_MARGIN
            }
            None => false,
        }
    }

    fn font_size(&self) -> f64 {
        FONT_SIZE_BASE + self.line_width * FONT_SIZE_PER_WIDTH
    }

    fn layout(&self, ctx: &Context) -> pango::Layout {
        let layout = pango::Layout::new(&pangocairo::functions::create_context(ctx));
        let mut font_desc = pango::FontDescription::from_string("Sans Bold");
        font_desc.set_size((self.font_size() * f64::from(pango::SCALE)) as i32);
        layout.set_font_description(Some(&font_desc));
        layout.set_text(&self.text);
        layout
    }
}

/// A contrasting halo keeps the text readable on any background: dark around light
/// colors, light around dark ones.
fn outline_color(color: colors::Color) -> colors::Color {
    let luminance = 0.2126 * color.red() + 0.7152 * color.green() + 0.0722 * color.blue();
    if luminance > 0.15 {
        colors::Color::new(0.0, 0.0, 0.0, 0.7)
    } else {
        colors::Color::new(1.0, 1.0, 1.0, 0.8)
    }
}

impl DrawingTool for TextLabel {
    fn press_mouse(&mut self, point: Point) {
        if self.position.is_none() {
            self.position = Some(point);
            self.editing = true;
        }
    }

    fn release_mouse(&mut self, _: Point) {
        self.drag_offset = None;
    }

    fn motion_notify(&mut self, point: Point) {
        if let Some(offset) = self.drag_offset {
            self.position = Some(point + offset);
        }
    }

    fn draw(&self, ctx: &Context) {
        let Some(pos) = self.position else {
            return;
        };
        if self.text.is_empty() && !self.editing {
            self.bounds.set(None);
            return;
        }

        let layout = self.layout(ctx);
        let first_line_height = layout
            .line_readonly(0)
            .map(|line| f64::from(line.pixel_extents().1.height()))
            .unwrap_or(0.0);
        let left = pos.0;
        let top = pos.1 - first_line_height / 2.0;

        let (_, logical) = layout.pixel_extents();
        self.bounds.set(Some((
            left + f64::from(logical.x()),
            top + f64::from(logical.y()),
            f64::from(logical.width()),
            f64::from(logical.height()),
        )));

        if !self.text.is_empty() {
            ctx.new_path();
            ctx.move_to(left, top);
            pangocairo::functions::layout_path(ctx, &layout);
            set_source_color(
                ctx,
                outline_color(self.color),
                f64::from(self.color.alpha()),
            );
            ctx.set_line_join(gtk::cairo::LineJoin::Round);
            ctx.set_line_width((self.font_size() / 8.0).max(2.0));
            report(ctx.stroke_preserve());
            set_source_color(ctx, self.color, 1.0);
            report(ctx.fill());
        }

        if self.editing {
            // Caret after the last character (on the last line for multi-line text).
            let (strong, _) = layout.cursor_pos(self.text.len() as i32);
            let scale = f64::from(pango::SCALE);
            let x = left + f64::from(strong.x()) / scale + 2.0;
            let y = top + f64::from(strong.y()) / scale;
            let h = f64::from(strong.height()) / scale;
            ctx.new_path();
            set_source_color(ctx, self.color, 1.0);
            ctx.set_line_cap(gtk::cairo::LineCap::Butt);
            ctx.set_line_width((self.font_size() / 14.0).max(2.0));
            ctx.move_to(x, y);
            ctx.line_to(x, y + h);
            report(ctx.stroke());
        }
    }

    fn set_line_width(&mut self, width: f64) {
        self.line_width = width;
    }

    fn set_color(&mut self, color: colors::Color) {
        self.color = color;
    }

    fn active(&mut self) -> bool {
        self.editing || self.drag_offset.is_some()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}
