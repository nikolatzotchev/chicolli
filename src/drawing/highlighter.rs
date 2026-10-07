use std::any::Any;

use gtk::cairo::Context;

use crate::colors;

use super::drawing_tool::{set_source_color, stroke_relaxed_path, DrawingTool, Point};
use super::normal_line::Freehand;

/// Opacity applied on top of the chosen color's own alpha.
const ALPHA: f64 = 0.4;
/// The highlighter is this many times wider than the selected line width, so the
/// default width of 5 gives the classic 20 px marker.
pub const WIDTH_FACTOR: f64 = 4.0;

pub struct Highlighter {
    stroke: Freehand,
    line_width: f64,
    color: colors::Color,
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl Highlighter {
    pub fn new() -> Highlighter {
        Highlighter {
            stroke: Freehand::default(),
            line_width: 20.0,
            color: colors::YELLOW,
        }
    }
}

impl DrawingTool for Highlighter {
    fn release_mouse(&mut self, point: Point) {
        self.stroke.release(point);
    }

    fn press_mouse(&mut self, point: Point) {
        self.stroke.press(point);
    }

    fn motion_notify(&mut self, point: Point) {
        self.stroke.motion(point);
    }

    fn draw(&self, ctx: &Context) {
        // Draw the stroke as one path so overlapping parts do not stack up darker.
        set_source_color(ctx, self.color, ALPHA);
        stroke_relaxed_path(ctx, self.stroke.points(), self.line_width);
    }

    fn set_line_width(&mut self, width: f64) {
        self.line_width = width * WIDTH_FACTOR;
        self.stroke.set_min_distance(self.line_width / 2.0);
    }

    fn set_color(&mut self, color: colors::Color) {
        self.color = color;
    }

    fn active(&mut self) -> bool {
        self.stroke.active()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn hit(&self, point: Point) -> bool {
        self.stroke.hit(point, self.line_width)
    }

    fn translate(&mut self, by: Point) {
        self.stroke.translate(by);
    }

    fn bounds(&self) -> Option<(Point, Point)> {
        self.stroke.bounds(self.line_width)
    }
}
