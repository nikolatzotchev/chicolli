use std::any::Any;

use gtk::cairo::Context;

use crate::colors;

use super::drawing_tool::{DrawingTool, Point};
use super::normal_line::calc_whole_spline;

pub struct Highlighter {
    points: Vec<Point>,
    finished: bool,
    started: bool,
    line_width: f64,
    color: colors::Color,
    alpha: f64,
}

impl Highlighter {
    pub fn new() -> Highlighter {
        Highlighter {
            points: Vec::new(),
            finished: false,
            started: false,
            line_width: 40.0,
            color: colors::RED,
            alpha: 0.4,
        }
    }
}

impl DrawingTool for Highlighter {
    fn release_mouse(&mut self, _: Point) {
        self.finished = true;
    }

    fn press_mouse(&mut self, _: Point) {
        self.started = true;
    }

    fn motion_notify(&mut self, point: Point) {
        if self.active() {
            self.points.push(point);
        }
    }

    fn draw(&self, ctx: &Context) {
        let color = self.color;
        ctx.set_source_rgba(
            color.red().into(),
            color.green().into(),
            color.blue().into(),
            self.alpha,
        );
        ctx.set_line_width(self.line_width);
        ctx.set_line_cap(gtk::cairo::LineCap::Round);
        ctx.set_line_join(gtk::cairo::LineJoin::Round);

        if self.points.len() > 3 {
            let controls = calc_whole_spline(&self.points);
            let first_point = self.points[0];
            ctx.move_to(first_point.0, first_point.1);
            for i in 0..self.points.len() - 2 {
                let p_0 = self.points[i];
                let p_1 = self.points[i + 1];
                ctx.curve_to(
                    p_0.0 + controls[i].0,
                    p_0.1 + controls[i].1,
                    p_1.0 - controls[i + 1].0,
                    p_1.1 - controls[i + 1].1,
                    p_1.0,
                    p_1.1,
                )
            }
            match ctx.stroke() {
                Err(e) => panic!("{e}"),
                _ => (),
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
        return self.started && !self.finished;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
