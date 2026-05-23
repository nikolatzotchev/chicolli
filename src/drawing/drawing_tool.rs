use std::any::Any;

use gtk::cairo::Context;

use crate::colors;
pub use crate::geometry::{snap_angle, snap_square, Point};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentDrawingTool {
    NormalLine,
    NormalArrowHeadBase,
    NormalArrowHeadPointer,
    NormalRectangle,
    Highlighter,
    TextLabel,
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
