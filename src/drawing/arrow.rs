use std::any::Any;

use crate::colors::{self, Color};
use crate::geometry::arrow_head;

use super::drawing_tool::{report, set_source_color, snap_angle, DrawingTool, Point};

/// Half the opening angle of the arrowhead (about 33 degrees).
const HEAD_HALF_ANGLE: f64 = 0.58067840828;
/// Smallest arrowhead length; thicker lines get proportionally bigger heads.
const MIN_HEAD_LENGTH: f64 = 20.0;
const HEAD_LENGTH_PER_WIDTH: f64 = 4.0;

pub struct NormalArrow {
    start: Option<Point>,
    end: Option<Point>,
    line_width: f64,
    finished: bool,
    direction_head_base: bool,
    color: Color,
    constrained: bool,
}

impl NormalArrow {
    pub fn new(direction: bool) -> NormalArrow {
        NormalArrow {
            start: None,
            end: None,
            line_width: 5.0,
            finished: false,
            direction_head_base: direction,
            color: colors::RED,
            constrained: false,
        }
    }

    fn resolved_end(&self) -> Option<(Point, Point)> {
        let (start, end) = (self.start?, self.end?);
        let end = if self.constrained {
            snap_angle(start, end)
        } else {
            end
        };
        Some((start, end))
    }
}

impl DrawingTool for NormalArrow {
    fn release_mouse(&mut self, point: Point) {
        if self.active() {
            self.end = Some(point);
            self.finished = true;
        }
    }

    fn press_mouse(&mut self, point: Point) {
        self.start = Some(point);
    }

    fn motion_notify(&mut self, point: Point) {
        if self.active() {
            self.end = Some(point)
        }
    }

    fn draw(&self, cnx: &gtk::cairo::Context) {
        let Some((start, end)) = self.resolved_end() else {
            return;
        };
        let (tail, tip) = if self.direction_head_base {
            (end, start)
        } else {
            (start, end)
        };
        // A click without a drag has no direction to point in.
        let head_length = MIN_HEAD_LENGTH.max(self.line_width * HEAD_LENGTH_PER_WIDTH);
        let Some((wing1, wing2)) = arrow_head(tail, tip, head_length, HEAD_HALF_ANGLE) else {
            return;
        };

        set_source_color(cnx, self.color, 1.0);
        cnx.set_line_cap(gtk::cairo::LineCap::Round);
        cnx.set_line_join(gtk::cairo::LineJoin::Round);
        cnx.set_line_width(self.line_width);
        cnx.move_to(tail.0, tail.1);
        cnx.line_to(tip.0, tip.1);
        // Draw the head as one polyline so its joint is rounded like the rest of the arrow.
        cnx.move_to(wing1.0, wing1.1);
        cnx.line_to(tip.0, tip.1);
        cnx.line_to(wing2.0, wing2.1);
        report(cnx.stroke());
    }

    fn set_line_width(&mut self, width: f64) {
        self.line_width = width;
    }

    fn set_color(&mut self, color: crate::colors::Color) {
        self.color = color;
    }

    fn active(&mut self) -> bool {
        self.start.is_some() && !self.finished
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn set_constrained(&mut self, constrained: bool) {
        self.constrained = constrained;
    }

    fn is_empty(&self) -> bool {
        match self.resolved_end() {
            Some((start, end)) => start.0 == end.0 && start.1 == end.1,
            None => true,
        }
    }
}
