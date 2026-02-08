use std::any::Any;

use gtk::cairo::Context;

use crate::colors;

use super::drawing_tool::{DrawingTool, Point};

const MIN_POINT_DISTANCE_SQ: f64 = 4.0;

fn distance_sq(a: &Point, b: &Point) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    dx * dx + dy * dy
}

pub struct NormalLine {
    points: Vec<Point>,
    finished: bool,
    started: bool,
    line_width: f64,
    color: colors::Color,
}

impl NormalLine {
    pub fn new() -> NormalLine {
        NormalLine {
            points: Vec::new(),
            finished: false,
            started: false,
            line_width: 5.0,
            color: colors::RED,
        }
    }
}

// https://www.ibiblio.org/e-notes/Splines/b-int.html
pub fn calc_whole_spline(points: &[Point]) -> Vec<Point> {
    let num_points = points.len();
    if num_points < 4 {
        return vec![];
    }
    let mut a = vec![Point(0.0, 0.0); num_points];
    let mut b = vec![0.0; num_points];
    b[1] = -0.25;
    let mut d = vec![Point(0.0, 0.0); num_points];
    d[0] = (points[1] - points[0]) / 3.0;
    d[num_points - 1] = (points[num_points - 1] - points[num_points - 2]) / 3.0;

    a[1] = (points[2] - points[0] - d[0]) / 4.0;
    for i in 2..num_points - 1 {
        b[i] = -1.0 / (4.0 + b[i - 1]);
        a[i] = -(points[i + 1] - points[i - 1] - a[i - 1]) * b[i];
    }
    for i in (1..num_points - 2).rev() {
        d[i] = a[i] + d[i + 1] * b[i];
    }
    d
}

impl DrawingTool for NormalLine {
    fn release_mouse(&mut self, point: Point) {
        if self.active() && (self.points.is_empty() || distance_sq(self.points.last().unwrap(), &point) > 0.0) {
            self.points.push(point);
        }
        self.finished = true;
    }

    fn press_mouse(&mut self, point: Point) {
        self.started = true;
        self.points.push(point);
    }

    fn motion_notify(&mut self, point: Point) {
        if self.active() {
            if let Some(last) = self.points.last() {
                if distance_sq(last, &point) < MIN_POINT_DISTANCE_SQ {
                    return;
                }
            }
            self.points.push(point);
        }
    }

    fn draw(&self, ctx: &Context) {
        if self.points.is_empty() {
            return;
        }

        let color = self.color;
        ctx.set_source_rgb(
            color.red().into(),
            color.green().into(),
            color.blue().into(),
        );
        ctx.set_line_width(self.line_width);
        ctx.set_line_cap(gtk::cairo::LineCap::Round);
        ctx.set_line_join(gtk::cairo::LineJoin::Round);

        let n = self.points.len();

        if n == 1 {
            let p = self.points[0];
            ctx.arc(p.0, p.1, self.line_width / 2.0, 0.0, std::f64::consts::TAU);
            if let Err(e) = ctx.fill() {
                panic!("{e}");
            }
            return;
        }

        if n == 2 {
            ctx.move_to(self.points[0].0, self.points[0].1);
            ctx.line_to(self.points[1].0, self.points[1].1);
            if let Err(e) = ctx.stroke() {
                panic!("{e}");
            }
            return;
        }

        if n == 3 {
            let p0 = self.points[0];
            let p1 = self.points[1];
            let p2 = self.points[2];
            ctx.move_to(p0.0, p0.1);
            ctx.curve_to(
                p0.0 + (p1.0 - p0.0) * 0.5,
                p0.1 + (p1.1 - p0.1) * 0.5,
                p1.0 + (p2.0 - p1.0) * 0.5,
                p1.1 + (p2.1 - p1.1) * 0.5,
                p2.0,
                p2.1,
            );
            if let Err(e) = ctx.stroke() {
                panic!("{e}");
            }
            return;
        }

        let controls = calc_whole_spline(&self.points);
        let first_point = self.points[0];
        ctx.move_to(first_point.0, first_point.1);
        for i in 0..n - 1 {
            let p_0 = self.points[i];
            let p_1 = self.points[i + 1];
            ctx.curve_to(
                p_0.0 + controls[i].0,
                p_0.1 + controls[i].1,
                p_1.0 - controls[i + 1].0,
                p_1.1 - controls[i + 1].1,
                p_1.0,
                p_1.1,
            );
        }
        if let Err(e) = ctx.stroke() {
            panic!("{e}");
        }
    }

    fn set_line_width(&mut self, width: f64) {
        self.line_width = width;
    }

    fn set_color(&mut self, color: colors::Color) {
        self.color = color;
    }

    fn active(&mut self) -> bool {
        self.started && !self.finished
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
