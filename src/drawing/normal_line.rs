use gtk::cairo::Context;

use crate::colors;

use super::drawing_tool::{DrawingTool, Point};

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
            line_width: 2.0,
            color: colors::RED,
        }
    }
}
// https://www.ibiblio.org/e-notes/Splines/b-int.html
pub fn calc_whole_spline(points: &Vec<Point>) -> Vec<Point> {
    let num_points = points.len();
    // it does not work with less than 4 points, but this does not affect us since adding
    // points happens fast
    if num_points < 4 {
        return vec![];
    }
    let mut a = vec![Point(0.0, 0.0); num_points];
    let mut b = vec![0.0; num_points];
    b[1] = -0.25;
    let mut d = vec![Point(0.0, 0.0); num_points];
    // maybe try to set these values better in the future
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

    fn draw(&self, ctx: &Context) -> () {
        let color = self.color;
        ctx.set_source_rgb(
            color.red().into(),
            color.green().into(),
            color.blue().into(),
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
}
