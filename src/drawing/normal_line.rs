use std::any::Any;

use gtk::cairo::Context;

use crate::colors;
use crate::geometry::distance_sq;

use super::drawing_tool::{set_source_color, stroke_smooth_path, DrawingTool, Point};

/// Motion events closer than this (in px) to the last kept point are skipped, so pointer
/// jitter does not put tiny wiggles into the spline.
pub const MIN_POINT_DISTANCE: f64 = 2.0;

/// The points of a freehand stroke, shared by the pen and the highlighter.
pub struct Freehand {
    points: Vec<Point>,
    started: bool,
    finished: bool,
    min_distance_sq: f64,
}

impl Default for Freehand {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            started: false,
            finished: false,
            min_distance_sq: MIN_POINT_DISTANCE * MIN_POINT_DISTANCE,
        }
    }
}

impl Freehand {
    /// Wide strokes need sparser points: jitter smaller than the stroke's half width
    /// bends the curve tighter than the pen, which shows up as scalloped edges.
    pub fn set_min_distance(&mut self, distance: f64) {
        let distance = distance.max(MIN_POINT_DISTANCE);
        self.min_distance_sq = distance * distance;
    }

    pub fn press(&mut self, point: Point) {
        self.started = true;
        self.points.push(point);
    }

    pub fn motion(&mut self, point: Point) {
        if !self.active() {
            return;
        }
        if let Some(&last) = self.points.last() {
            if distance_sq(last, point) < self.min_distance_sq {
                return;
            }
        }
        self.points.push(point);
    }

    pub fn release(&mut self, point: Point) {
        if !self.active() {
            return;
        }
        match self.points.last() {
            Some(&last) if distance_sq(last, point) < self.min_distance_sq => {
                // Snap the end of the stroke to where the button came up instead of
                // adding a near-duplicate point that would kink the curve.
                if self.points.len() > 1 {
                    *self.points.last_mut().unwrap() = point;
                }
            }
            _ => self.points.push(point),
        }
        self.finished = true;
    }

    pub fn active(&self) -> bool {
        self.started && !self.finished
    }

    pub fn points(&self) -> &[Point] {
        &self.points
    }
}

pub struct NormalLine {
    stroke: Freehand,
    line_width: f64,
    color: colors::Color,
}

impl Default for NormalLine {
    fn default() -> Self {
        Self::new()
    }
}

impl NormalLine {
    pub fn new() -> NormalLine {
        NormalLine {
            stroke: Freehand::default(),
            line_width: 5.0,
            color: colors::RED,
        }
    }
}

impl DrawingTool for NormalLine {
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
        set_source_color(ctx, self.color, 1.0);
        stroke_smooth_path(ctx, self.stroke.points(), self.line_width);
    }

    fn set_line_width(&mut self, width: f64) {
        self.line_width = width;
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
}
