use gtk::cairo::Context;

use super::drawing_tool::{report, Point};

/// A press and release closer together than this (in px) is a click, which deletes the
/// element, rather than a drag, which moves it.
pub const DRAG_THRESHOLD: f64 = 4.0;
/// Room between an element and the box drawn around it.
const PADDING: f64 = 4.0;

/// Outlines the element the select tool would pick up: a dashed box, dark under light so
/// it shows on any background.
pub fn draw_outline(ctx: &Context, (min, max): (Point, Point)) {
    ctx.save().ok();
    ctx.new_path();
    ctx.rectangle(
        min.0 - PADDING,
        min.1 - PADDING,
        max.0 - min.0 + 2.0 * PADDING,
        max.1 - min.1 + 2.0 * PADDING,
    );
    ctx.set_source_rgba(0.0, 0.0, 0.0, 0.5);
    ctx.set_line_width(3.0);
    report(ctx.stroke_preserve());
    ctx.set_source_rgba(1.0, 1.0, 1.0, 0.95);
    ctx.set_line_width(1.5);
    ctx.set_dash(&[6.0, 4.0], 0.0);
    report(ctx.stroke());
    ctx.restore().ok();
}
