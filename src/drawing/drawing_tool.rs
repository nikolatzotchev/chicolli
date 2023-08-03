use gtk4::cairo::Context;

use crate::colors;

#[derive(Clone, Debug, Copy)]
pub struct Point(pub f64,pub f64);

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

impl std::ops::Neg for Point{
    type Output = Point;

    fn neg(self) -> Point{
        Point(-self.0, -self.1)
    }
}

pub enum CurrentDrawingTool {
    NormalLine,
    NormalArrow,
}

pub trait DrawingTool {
    fn release_mouse(&mut self, point: Point);
    fn press_mouse(&mut self, point: Point);
    fn motion_notify(&mut self, point: Point);
    fn draw(&self, cnx: &Context);
    fn set_line_width(&mut self, width: f64);
    fn set_color(&mut self, color: colors::Color);
    fn active(&mut self) -> bool;
}

