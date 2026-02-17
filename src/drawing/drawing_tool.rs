use std::any::Any;

use gtk::cairo::Context;

use crate::colors;

#[derive(Clone, Debug, Copy)]
pub struct Point(pub f64, pub f64);

impl std::ops::Add<Point> for Point {
    type Output = Point;

    fn add(self, rhs: Point) -> Self::Output {
        Point(self.0 + rhs.0, self.1 + rhs.1)
    }
}
impl std::ops::Sub<Point> for Point {
    type Output = Point;

    fn sub(self, rhs: Point) -> Self::Output {
        Point(self.0 - rhs.0, self.1 - rhs.1)
    }
}
impl std::ops::Mul<f64> for Point {
    type Output = Point;

    fn mul(self, rhs: f64) -> Self::Output {
        Point(self.0 * rhs, self.1 * rhs)
    }
}

impl std::ops::Div<f64> for Point {
    type Output = Point;

    fn div(self, rhs: f64) -> Self::Output {
        Point(self.0 / rhs, self.1 / rhs)
    }
}

impl std::ops::Neg for Point {
    type Output = Point;

    fn neg(self) -> Point {
        Point(-self.0, -self.1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentDrawingTool {
    NormalLine,
    NormalArrowHeadBase,
    NormalArrowHeadPointer,
    NormalRectangle,
    Highlighter,
    TextLabel,
}

pub fn snap_angle(start: Point, end: Point) -> Point {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let angle = dy.atan2(dx);
    let snapped = (angle / std::f64::consts::FRAC_PI_4).round() * std::f64::consts::FRAC_PI_4;
    let len = (dx * dx + dy * dy).sqrt();
    Point(start.0 + len * snapped.cos(), start.1 + len * snapped.sin())
}

pub fn snap_square(start: Point, end: Point) -> Point {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let side = dx.abs().max(dy.abs());
    Point(start.0 + side * dx.signum(), start.1 + side * dy.signum())
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
}
