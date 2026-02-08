use std::any::Any;

use crate::colors;

use super::drawing_tool::{DrawingTool, Point};

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
        if let Some(pos) = self.position {
            if !self.text.is_empty() {
                let color = self.color;
                ctx.set_source_rgb(
                    color.red().into(),
                    color.green().into(),
                    color.blue().into(),
                );
                ctx.set_font_size(self.line_width * 8.0);
                ctx.select_font_face(
                    "Sans",
                    gtk::cairo::FontSlant::Normal,
                    gtk::cairo::FontWeight::Normal,
                );
                ctx.move_to(pos.0, pos.1);
                if let Err(e) = ctx.show_text(&self.text) {
                    println!("{e}")
                }
            }
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
}
