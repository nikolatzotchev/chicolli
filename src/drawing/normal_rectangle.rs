use std::any::Any;

use crate::colors;

use super::drawing_tool::{report, set_source_color, snap_square, DrawingTool, Point};

pub struct NormalRectangle {
    start: Option<Point>,
    end: Option<Point>,
    finished: bool,
    line_width: f64,
    color: colors::Color,
    constrained: bool,
}

impl Default for NormalRectangle {
    fn default() -> Self {
        Self::new()
    }
}

impl NormalRectangle {
    pub fn new() -> NormalRectangle {
        NormalRectangle {
            start: None,
            end: None,
            finished: false,
            line_width: 5.0,
            color: colors::RED,
            constrained: false,
        }
    }

    fn resolved_end(&self) -> Option<(Point, Point)> {
        let (start, end) = (self.start?, self.end?);
        let end = if self.constrained {
            snap_square(start, end)
        } else {
            end
        };
        Some((start, end))
    }
}

impl DrawingTool for NormalRectangle {
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
            self.end = Some(point);
        }
    }

    fn draw(&self, cnx: &gtk::cairo::Context) {
        let Some((start, end)) = self.resolved_end() else {
            return;
        };
        set_source_color(cnx, self.color, 1.0);
        cnx.set_line_cap(gtk::cairo::LineCap::Round);
        cnx.set_line_join(gtk::cairo::LineJoin::Round);
        cnx.set_line_width(self.line_width);
        cnx.rectangle(
            f64::min(start.0, end.0),
            f64::min(start.1, end.1),
            (end.0 - start.0).abs(),
            (end.1 - start.1).abs(),
        );
        report(cnx.stroke());
    }

    fn set_line_width(&mut self, width: f64) {
        self.line_width = width;
    }

    fn set_color(&mut self, color: crate::colors::Color) {
        self.color = color
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
