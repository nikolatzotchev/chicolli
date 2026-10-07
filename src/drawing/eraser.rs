use std::any::Any;

use gtk::cairo::{Context, Operator};

use crate::colors;

use super::drawing_tool::{report, stroke_smooth_path, DrawingTool, Point};
use super::normal_line::Freehand;

/// The eraser is this many times wider than the selected line width, like the
/// highlighter, so the default width of 5 erases a 20 px path.
pub const WIDTH_FACTOR: f64 = 4.0;

/// A freehand stroke that clears what was drawn before it, down to the desktop. It is an
/// element like any other, so Undo brings back what it erased and anything drawn later
/// goes on top of it.
pub struct Eraser {
    stroke: Freehand,
    line_width: f64,
}

impl Default for Eraser {
    fn default() -> Self {
        Self::new()
    }
}

impl Eraser {
    pub fn new() -> Eraser {
        Eraser {
            stroke: Freehand::default(),
            line_width: 20.0,
        }
    }
}

/// How wide the eraser is at line width `width`.
pub fn eraser_width(width: f64) -> f64 {
    width * WIDTH_FACTOR
}

/// Outlines the eraser's reach around `center`, so its size shows before erasing.
pub fn draw_outline(ctx: &Context, center: Point, width: f64) {
    let radius = eraser_width(width) / 2.0;
    ctx.save().ok();
    ctx.new_path();
    ctx.arc(center.0, center.1, radius, 0.0, std::f64::consts::TAU);
    // A white ring inside a faint dark rim reads on light and dark backgrounds.
    ctx.set_source_rgba(0.0, 0.0, 0.0, 0.45);
    ctx.set_line_width(3.0);
    report(ctx.stroke_preserve());
    ctx.set_source_rgba(1.0, 1.0, 1.0, 0.95);
    ctx.set_line_width(1.5);
    report(ctx.stroke());
    ctx.restore().ok();
}

impl DrawingTool for Eraser {
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
        ctx.save().ok();
        ctx.set_operator(Operator::Clear);
        stroke_smooth_path(ctx, self.stroke.points(), self.line_width);
        ctx.restore().ok();
    }

    fn set_line_width(&mut self, width: f64) {
        self.line_width = eraser_width(width);
        self.stroke.set_min_distance(self.line_width / 4.0);
    }

    fn set_color(&mut self, _color: colors::Color) {}

    fn active(&mut self) -> bool {
        self.stroke.active()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
