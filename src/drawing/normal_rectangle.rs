use crate::colors;

use super::drawing_tool::{DrawingTool, Point};

pub struct NormalRectangle {
    start: Option<Point>,
    end: Option<Point>,
    finished: bool,
    line_width: f64,
    color: colors::Color,
}

impl NormalRectangle {
    pub fn new() -> NormalRectangle {
        NormalRectangle {
            start: None,
            end: None,
            finished: false,
            line_width: 2.0,
            color: colors::RED,
        }
    }
}

impl DrawingTool for NormalRectangle {
    fn release_mouse(&mut self, point: Point) {
        self.end = Some(point);
        self.finished = true;
    }

    fn press_mouse(&mut self, point: Point) {
        self.start = Some(point);
    }

    fn motion_notify(&mut self, point: Point) {
        if !self.finished {
            self.end = Some(point);
        }
    }

    fn draw(&self, cnx: &gtk4::cairo::Context) {
        if let (Some(start), Some(end)) = (self.start, self.end) {
            let color = self.color;
            cnx.set_source_rgb(color.0, color.1, color.2);
            cnx.set_line_cap(gtk4::cairo::LineCap::Round);
            cnx.set_line_join(gtk4::cairo::LineJoin::Round);
            cnx.set_line_width(self.line_width);

            cnx.rectangle(
                f64::min(start.0, end.0),
                f64::min(start.1, end.1),
                (end.0 - start.0).abs(),
                (end.1 - start.1).abs(),
            );
        }

        match cnx.stroke() {
            Err(e) => println!("{e}"),
            _ => (),
        }
    }

    fn set_line_width(&mut self, width: f64) {
        self.line_width = width;
    }

    fn set_color(&mut self, color: crate::colors::Color) {
        self.color = color
    }

    fn active(&mut self) -> bool {
        return self.start.is_some() && !self.finished;
    }
}
