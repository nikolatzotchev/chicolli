use std::any::Any;

use gtk::cairo::Context;

use crate::colors;
use crate::geometry::spline_controls;
pub use crate::geometry::{snap_angle, snap_square, Point};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentDrawingTool {
    NormalLine,
    NormalArrowHeadBase,
    NormalArrowHeadPointer,
    NormalRectangle,
    Highlighter,
    TextLabel,
    Eraser,
}

pub trait DrawingTool {
    fn release_mouse(&mut self, point: Point);
    fn press_mouse(&mut self, point: Point);
    fn motion_notify(&mut self, point: Point);
    fn draw(&self, cnx: &Context);
    fn set_line_width(&mut self, width: f64);
    fn set_color(&mut self, color: colors::Color);
    fn active(&mut self) -> bool;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn set_constrained(&mut self, _constrained: bool) {}
    /// True when the element would draw nothing (a click without a drag, a label with
    /// no text). Such elements are dropped so Undo never removes something invisible.
    fn is_empty(&self) -> bool {
        false
    }
}

/// Sets `color` as the source, multiplying its own alpha by `alpha`.
pub fn set_source_color(ctx: &Context, color: colors::Color, alpha: f64) {
    ctx.set_source_rgba(
        color.red().into(),
        color.green().into(),
        color.blue().into(),
        f64::from(color.alpha()) * alpha,
    );
}

/// Strokes a smooth curve through `points` (a dot for a single point) with the
/// current source, using round caps and joins.
pub fn stroke_smooth_path(ctx: &Context, points: &[Point], line_width: f64) {
    ctx.set_line_width(line_width);
    ctx.set_line_cap(gtk::cairo::LineCap::Round);
    ctx.set_line_join(gtk::cairo::LineJoin::Round);

    match points {
        [] => return,
        [p] => {
            ctx.new_sub_path();
            ctx.arc(p.0, p.1, line_width / 2.0, 0.0, std::f64::consts::TAU);
            report(ctx.fill());
            return;
        }
        [p0, p1] => {
            ctx.move_to(p0.0, p0.1);
            ctx.line_to(p1.0, p1.1);
        }
        _ => {
            let controls = spline_controls(points);
            ctx.move_to(points[0].0, points[0].1);
            for i in 0..points.len() - 1 {
                let c0 = points[i] + controls[i];
                let c1 = points[i + 1] - controls[i + 1];
                let p = points[i + 1];
                ctx.curve_to(c0.0, c0.1, c1.0, c1.1, p.0, p.1);
            }
        }
    }
    report(ctx.stroke());
}

/// Strokes a curve that follows `points` loosely: it passes through the midpoints between
/// them and uses the points themselves as quadratic control points. Unlike
/// [`stroke_smooth_path`] it does not reproduce every wobble of the pointer, which matters
/// for wide strokes where small wobbles show up as bumpy edges.
pub fn stroke_relaxed_path(ctx: &Context, points: &[Point], line_width: f64) {
    if points.len() < 3 {
        stroke_smooth_path(ctx, points, line_width);
        return;
    }
    ctx.set_line_width(line_width);
    ctx.set_line_cap(gtk::cairo::LineCap::Round);
    ctx.set_line_join(gtk::cairo::LineJoin::Round);

    let mut from = points[0];
    ctx.move_to(from.0, from.1);
    for i in 1..points.len() - 1 {
        let control = points[i];
        let to = (points[i] + points[i + 1]) / 2.0;
        // Quadratic Bezier expressed as a cubic.
        let c0 = from + (control - from) * (2.0 / 3.0);
        let c1 = to + (control - to) * (2.0 / 3.0);
        ctx.curve_to(c0.0, c0.1, c1.0, c1.1, to.0, to.1);
        from = to;
    }
    let last = points[points.len() - 1];
    ctx.line_to(last.0, last.1);
    report(ctx.stroke());
}

/// Logs a Cairo drawing error instead of taking the whole overlay down with it.
pub fn report(result: Result<(), gtk::cairo::Error>) {
    if let Err(e) = result {
        eprintln!("chicolli: drawing error: {e}");
    }
}
