use crate::colors::{Color, self};

use super::drawing_tool::{DrawingTool, Point};

pub struct NormalArrow {
    start: Option<Point>,
    end: Option<Point>,
    arrow_length: f64,
    arrow_degree: f64,
    arrow_width: f64,
    finished: bool,
    color: Color
}

impl NormalArrow {
    pub fn new() -> NormalArrow {
        NormalArrow {
            start: None,
            end: None,
            arrow_length: 20.0,
            arrow_degree: 0.58067840828,
            arrow_width: 2.0,
            finished: false,
            color: colors::RED
        }
    }
}


impl DrawingTool for NormalArrow {
    fn release_mouse(&mut self, point: super::drawing_tool::Point) {
        self.end = Some(point);
        self.finished = true;
    }

    fn press_mouse(&mut self, point: super::drawing_tool::Point) {
        self.start = Some(point);
    }

    fn motion_notify(&mut self, point: super::drawing_tool::Point) {
        if !self.finished {
            self.end = Some(point)
        }
    }

    fn draw(&self, cnx: &gtk4::cairo::Context) {
        if let (Some(start), Some(end)) = (self.start, self.end) {
            let color = self.color;
            cnx.set_source_rgb(color.0, color.1, color.2);
            cnx.set_line_cap(gtk4::cairo::LineCap::Round); 
            cnx.set_line_join(gtk4::cairo::LineJoin::Round);
            cnx.set_line_width(self.arrow_width);
            cnx.move_to(start.0, start.1);
            cnx.line_to(end.0, end.1);

            let angle_main_line = (end.1 - start.1).atan2(end.0 - start.0);

            let (x1, y1): (f64, f64);

            let (x2, y2): (f64, f64);
            x1 = start.0 + self.arrow_length * (angle_main_line - self.arrow_degree).cos();
            y1 = start.1 + self.arrow_length * (angle_main_line - self.arrow_degree).sin();
            x2 = start.0 + self.arrow_length * (angle_main_line + self.arrow_degree).cos();
            y2 = start.1 + self.arrow_length * (angle_main_line + self.arrow_degree).sin();

            cnx.move_to(start.0, start.1);
            cnx.line_to(x1, y1);

            cnx.move_to(start.0, start.1);
            cnx.line_to(x2, y2);

            match cnx.stroke() {
                Err(e) => println!("{e}"),
                _ => ()
            }

        }
    }

    fn set_line_width(&mut self, width: f64) {
        self.arrow_width = width;
    }

    fn set_color(&mut self, color: crate::colors::Color) {
        self.color = color;
    }

    fn active(&mut self) -> bool {
        return self.start.is_some() && !self.finished;
    }
}

