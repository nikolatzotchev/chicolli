use std::any::Any;

use crate::colors;

use super::drawing_tool::{report, set_source_color, DrawingTool, Point};

/// Font size in points per unit of line width.
const FONT_SIZE_PER_WIDTH: f64 = 8.0;

pub struct TextLabel {
    position: Option<Point>,
    text: String,
    finished: bool,
    placed: bool,
    movable: bool,
    line_width: f64,
    color: colors::Color,
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
            finished: false,
            placed: false,
            movable: false,
            line_width: 5.0,
            color: colors::RED,
        }
    }

    pub fn push_char(&mut self, c: char) {
        self.text.push(c);
    }

    pub fn pop_char(&mut self) {
        self.text.pop();
    }

    pub fn finish(&mut self) {
        self.finished = true;
        self.movable = true;
    }

    /// Ends editing and leaves the label where it is (used when another tool takes over).
    pub fn commit(&mut self) {
        self.finished = true;
        self.movable = false;
    }

    pub fn is_editing(&self) -> bool {
        self.placed && !self.finished
    }

    pub fn finalize(&mut self) {
        self.movable = false;
    }

    pub fn is_movable(&self) -> bool {
        self.movable
    }

    pub fn is_placed(&self) -> bool {
        self.placed
    }
}

impl DrawingTool for TextLabel {
    fn press_mouse(&mut self, point: Point) {
        self.position = Some(point);
        self.placed = true;
    }

    fn release_mouse(&mut self, _: Point) {}

    fn motion_notify(&mut self, point: Point) {
        if self.movable {
            self.position = Some(point);
        }
    }

    fn draw(&self, ctx: &gtk::cairo::Context) {
        let Some(pos) = self.position else {
            return;
        };
        let editing = self.is_editing();
        if self.text.is_empty() && !editing {
            return;
        }
        set_source_color(ctx, self.color, 1.0);

        let layout = gtk::pango::Layout::new(&pangocairo::functions::create_context(ctx));
        let mut font_desc = gtk::pango::FontDescription::from_string("Sans");
        font_desc.set_size(
            (self.line_width * FONT_SIZE_PER_WIDTH * f64::from(gtk::pango::SCALE)) as i32,
        );
        layout.set_font_description(Some(&font_desc));
        layout.set_text(&self.text);

        ctx.move_to(pos.0, pos.1);
        pangocairo::functions::show_layout(ctx, &layout);

        if editing {
            // Text caret after the last character, so the user sees where typing goes.
            let (_, logical) = layout.pixel_extents();
            let caret_x = pos.0 + f64::from(logical.width()) + 2.0;
            ctx.new_path();
            ctx.set_line_width((self.line_width / 2.0).max(1.0));
            ctx.move_to(caret_x, pos.1);
            ctx.line_to(caret_x, pos.1 + f64::from(logical.height()));
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
        (self.placed && !self.finished) || self.movable
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}
