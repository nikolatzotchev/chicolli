//! Tool cursors, drawn at runtime with Cairo.
//!
//! Every cursor shares one look: dark ink strokes inside a white halo with a faint dark
//! rim, so it stays readable on light and dark backgrounds alike. The pen and highlighter
//! show the current drawing color, and the arrow and rectangle cursors pair a precision
//! crosshair with the same glyph as their toolbar button.
//!
//! Shapes are designed on a 32-unit grid and rendered at the device scale when GTK asks
//! for it (the `hidpi-cursors` feature, GTK 4.16+), so they stay sharp on HiDPI outputs.

use std::cell::Cell;
use std::f64::consts::{FRAC_PI_4, PI};

use gtk::cairo;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use crate::colors::Color;
use crate::drawing::drawing_tool::CurrentDrawingTool;
use crate::toolbar::{arrow_icon_path, rectangle_icon_path};

/// Side of the design grid every shape below is drawn on.
const GRID: f64 = 32.0;
/// Cursor size, in application pixels, when GTK does not ask for one.
const DEFAULT_SIZE: i32 = 32;

const INK: (f64, f64, f64) = (0.10, 0.10, 0.12);
/// Extra halo width around each ink stroke (split across both sides).
const HALO: f64 = 2.0;
/// Extra width of the faint dark rim outside the halo, for light backgrounds.
const RIM: f64 = 1.0;
const RIM_ALPHA: f64 = 0.35;
/// Ink width of the pen and highlighter outlines.
const OUTLINE: f64 = 1.5;
const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);

#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    Pen,
    Highlighter,
    Arrow { pointing_right: bool },
    Rectangle,
    Text,
}

impl Shape {
    fn for_tool(tool: CurrentDrawingTool) -> Self {
        match tool {
            CurrentDrawingTool::NormalLine => Shape::Pen,
            CurrentDrawingTool::Highlighter => Shape::Highlighter,
            CurrentDrawingTool::NormalArrowHeadPointer => Shape::Arrow {
                pointing_right: true,
            },
            CurrentDrawingTool::NormalArrowHeadBase => Shape::Arrow {
                pointing_right: false,
            },
            CurrentDrawingTool::NormalRectangle => Shape::Rectangle,
            CurrentDrawingTool::TextLabel => Shape::Text,
        }
    }

    /// Where the pointer sits on the grid. The compositor places the cursor so the
    /// pointer lands on this point (the hotspot pixel's top-left corner), and each shape
    /// puts its working point (pen tip, crosshair center, I-beam middle) exactly there.
    /// Two-unit strokes centered on it stay crisp at 1x.
    fn hotspot(self) -> (i32, i32) {
        match self {
            Shape::Pen => (4, 27),
            Shape::Highlighter => (5, 26),
            Shape::Arrow { .. } | Shape::Rectangle => (12, 12),
            Shape::Text => (15, 15),
        }
    }

    fn paint(self, cr: &cairo::Context, color: &Color) {
        cr.set_line_cap(cairo::LineCap::Round);
        cr.set_line_join(cairo::LineJoin::Round);
        let (hx, hy) = self.hotspot();
        let (x, y) = (f64::from(hx), f64::from(hy));
        match self {
            Shape::Pen => paint_pen(cr, x, y, color),
            Shape::Highlighter => paint_highlighter(cr, x, y, color),
            Shape::Arrow { pointing_right } => {
                paint_crosshair(cr, x, y);
                paint_badge(cr, false, |cr, s| arrow_icon_path(cr, s, pointing_right));
            }
            Shape::Rectangle => {
                paint_crosshair(cr, x, y);
                paint_badge(cr, true, rectangle_icon_path);
            }
            Shape::Text => paint_ibeam(cr, x, y),
        }
    }
}

fn set_ink(cr: &cairo::Context) {
    cr.set_source_rgb(INK.0, INK.1, INK.2);
}

/// Strokes the rim and halo behind a path whose ink will be `width` wide. `path` adds
/// the path to `cr` and is called once per layer.
fn backdrop(cr: &cairo::Context, width: f64, path: impl Fn(&cairo::Context)) {
    path(cr);
    cr.set_source_rgba(0.0, 0.0, 0.0, RIM_ALPHA);
    cr.set_line_width(width + HALO + RIM);
    let _ = cr.stroke();
    path(cr);
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.set_line_width(width + HALO);
    let _ = cr.stroke();
}

/// Strokes `path` in ink over its rim and halo.
fn inked(cr: &cairo::Context, width: f64, path: impl Fn(&cairo::Context)) {
    backdrop(cr, width, &path);
    path(cr);
    set_ink(cr);
    cr.set_line_width(width);
    let _ = cr.stroke();
}

/// Fills a closed `path` with `fill`, then outlines it in ink.
fn fill_and_outline(cr: &cairo::Context, fill: &Color, path: impl Fn(&cairo::Context)) {
    path(cr);
    cr.set_source_rgba(
        f64::from(fill.red()),
        f64::from(fill.green()),
        f64::from(fill.blue()),
        1.0,
    );
    let _ = cr.fill();
    path(cr);
    set_ink(cr);
    cr.set_line_width(OUTLINE);
    let _ = cr.stroke();
}

/// A pencil pointing down-left: tip on the hotspot, colored cone, white barrel.
fn paint_pen(cr: &cairo::Context, x: f64, y: f64, color: &Color) {
    cr.save().ok();
    cr.translate(x, y);
    cr.rotate(-FRAC_PI_4);

    const R: f64 = 3.5; // barrel half-width
    const CONE: f64 = 8.0;
    const END: f64 = 27.0;
    let silhouette = |cr: &cairo::Context| {
        cr.move_to(0.0, 0.0);
        cr.line_to(CONE, -R);
        cr.line_to(END, -R);
        cr.arc(END, 0.0, R, -PI / 2.0, PI / 2.0);
        cr.line_to(CONE, R);
        cr.close_path();
    };
    backdrop(cr, OUTLINE, silhouette);
    fill_and_outline(cr, &WHITE, silhouette);
    fill_and_outline(cr, color, |cr| {
        cr.move_to(0.0, 0.0);
        cr.line_to(CONE, -R);
        cr.line_to(CONE, R);
        cr.close_path();
    });
    // Ferrule line near the end of the barrel.
    set_ink(cr);
    cr.set_line_width(OUTLINE);
    cr.move_to(END - 4.0, -R);
    cr.line_to(END - 4.0, R);
    let _ = cr.stroke();

    cr.restore().ok();
}

/// A chisel marker like the toolbar's, its tip centered on the hotspot and filled with
/// the current color.
fn paint_highlighter(cr: &cairo::Context, x: f64, y: f64, color: &Color) {
    cr.save().ok();
    cr.translate(x, y);
    cr.rotate(-FRAC_PI_4);

    const TIP: f64 = 1.75; // half-width of the chisel's flat edge
    const R: f64 = 4.5; // body half-width
    const CHISEL: f64 = 5.0;
    const END: f64 = 27.0;
    let silhouette = |cr: &cairo::Context| {
        cr.move_to(0.0, -TIP);
        cr.line_to(CHISEL, -R);
        cr.line_to(END, -R);
        cr.line_to(END, R);
        cr.line_to(CHISEL, R);
        cr.line_to(0.0, TIP);
        cr.close_path();
    };
    // A marker's barrel is tinted with its ink, which also tells it apart from the pen.
    let tint = |c: f32| 1.0 - 0.4 * (1.0 - c);
    let barrel = Color::new(
        tint(color.red()),
        tint(color.green()),
        tint(color.blue()),
        1.0,
    );
    backdrop(cr, OUTLINE, silhouette);
    fill_and_outline(cr, &barrel, silhouette);
    fill_and_outline(cr, color, |cr| cr.rectangle(END - 5.0, -R, 5.0, 2.0 * R));
    fill_and_outline(cr, color, |cr| {
        cr.move_to(0.0, -TIP);
        cr.line_to(CHISEL, -R);
        cr.line_to(CHISEL, R);
        cr.line_to(0.0, TIP);
        cr.close_path();
    });
    cr.restore().ok();
}

/// Thin crosshair with an open center, so the exact target pixel stays visible.
fn paint_crosshair(cr: &cairo::Context, x: f64, y: f64) {
    const GAP: f64 = 4.0;
    const ARM: f64 = 7.5;
    // Square caps let the halo wrap the ends of each arm.
    cr.set_line_cap(cairo::LineCap::Square);
    inked(cr, 2.0, |cr| {
        for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            cr.move_to(x + dx * GAP, y + dy * GAP);
            cr.line_to(x + dx * ARM, y + dy * ARM);
        }
    });
    cr.set_line_cap(cairo::LineCap::Round);
}

/// The tool's toolbar glyph, small, below and right of the crosshair. A closed glyph is
/// filled white so its small inside stays clean instead of showing a blurred halo.
fn paint_badge(cr: &cairo::Context, closed: bool, glyph: impl Fn(&cairo::Context, f64)) {
    const ORIGIN: f64 = 15.0;
    const SIZE: f64 = 16.0;
    cr.save().ok();
    cr.translate(ORIGIN, ORIGIN);
    backdrop(cr, 2.0, |cr| glyph(cr, SIZE));
    if closed {
        glyph(cr, SIZE);
        cr.set_source_rgb(1.0, 1.0, 1.0);
        let _ = cr.fill();
    }
    glyph(cr, SIZE);
    set_ink(cr);
    cr.set_line_width(2.0);
    let _ = cr.stroke();
    cr.restore().ok();
}

/// I-beam centered on the hotspot: labels start at the click, vertically centered on
/// their first line.
fn paint_ibeam(cr: &cairo::Context, x: f64, y: f64) {
    const HALF_HEIGHT: f64 = 10.0;
    const SERIF: f64 = 3.0;
    cr.set_line_cap(cairo::LineCap::Square);
    inked(cr, 2.0, |cr| {
        cr.move_to(x, y - HALF_HEIGHT);
        cr.line_to(x, y + HALF_HEIGHT);
        for sy in [y - HALF_HEIGHT, y + HALF_HEIGHT] {
            cr.move_to(x - SERIF, sy);
            cr.line_to(x + SERIF, sy);
        }
    });
    cr.set_line_cap(cairo::LineCap::Round);
}

/// A rendered cursor: premultiplied ARGB pixels plus its logical size and hotspot.
struct Image {
    surface: cairo::ImageSurface,
    #[cfg_attr(not(feature = "hidpi-cursors"), allow(dead_code))]
    logical_size: i32,
    hotspot: (i32, i32),
}

/// Renders `shape` for a cursor `logical_size` application pixels wide at `scale`
/// device pixels per application pixel.
fn render(shape: Shape, color: &Color, logical_size: i32, scale: f64) -> Option<Image> {
    let px = (f64::from(logical_size) * scale).ceil().max(1.0) as i32;
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, px, px).ok()?;
    {
        let cr = cairo::Context::new(&surface).ok()?;
        cr.set_antialias(cairo::Antialias::Best);
        cr.scale(f64::from(px) / GRID, f64::from(px) / GRID);
        shape.paint(&cr, color);
    }
    let (hx, hy) = shape.hotspot();
    let to_logical = |v: i32| {
        let v = f64::from(v) * f64::from(logical_size) / GRID;
        (v.round() as i32).clamp(0, logical_size - 1)
    };
    Some(Image {
        surface,
        logical_size,
        hotspot: (to_logical(hx), to_logical(hy)),
    })
}

fn texture(image: Image) -> Option<gdk::Texture> {
    let mut surface = image.surface;
    surface.flush();
    let (w, h, stride) = (surface.width(), surface.height(), surface.stride());
    let bytes = glib::Bytes::from(surface.data().ok()?.as_ref());
    // Cairo's ARGB32 is native-endian premultiplied, i.e. BGRA bytes on little-endian.
    Some(
        gdk::MemoryTexture::new(
            w,
            h,
            gdk::MemoryFormat::B8g8r8a8Premultiplied,
            &bytes,
            stride as usize,
        )
        .upcast(),
    )
}

/// Cursors are a third larger than the theme's arrow so the tool glyphs stay legible;
/// the usual 24px theme size gives the 32px design size.
#[cfg(feature = "hidpi-cursors")]
fn logical_size(theme_size: i32) -> i32 {
    if theme_size <= 0 {
        return DEFAULT_SIZE;
    }
    (theme_size.max(16) * 4 + 2) / 3
}

#[cfg(feature = "hidpi-cursors")]
fn cursor_for(tool: CurrentDrawingTool, color: Color) -> Option<gdk::Cursor> {
    let shape = Shape::for_tool(tool);
    let fallback = gdk::Cursor::from_name("crosshair", None);
    gdk::Cursor::from_callback(
        move |_, theme_size, scale, width, height, hotspot_x, hotspot_y| {
            let size = logical_size(theme_size);
            let rendered = render(shape, &color, size, scale).and_then(|image| {
                let hotspot = image.hotspot;
                Some((image.logical_size, hotspot, texture(image)?))
            });
            // GTK needs a texture back; an empty one only happens if allocation failed.
            let (size, hotspot, texture) = rendered.unwrap_or_else(|| {
                let bytes = glib::Bytes::from_static(&[0; 4]);
                let empty = gdk::MemoryTexture::new(
                    1,
                    1,
                    gdk::MemoryFormat::B8g8r8a8Premultiplied,
                    &bytes,
                    4,
                );
                (1, (0, 0), empty.upcast())
            });
            *width = size;
            *height = size;
            (*hotspot_x, *hotspot_y) = hotspot;
            texture
        },
        fallback.as_ref(),
    )
}

#[cfg(not(feature = "hidpi-cursors"))]
fn cursor_for(tool: CurrentDrawingTool, color: Color) -> Option<gdk::Cursor> {
    let image = render(Shape::for_tool(tool), &color, DEFAULT_SIZE, 1.0)?;
    let (hx, hy) = image.hotspot;
    Some(gdk::Cursor::from_texture(&texture(image)?, hx, hy, None))
}

/// Keeps a widget's cursor matched to the current tool and color.
pub struct ToolCursor {
    widget: gtk::Widget,
    shown: Cell<Option<(CurrentDrawingTool, Color)>>,
}

impl ToolCursor {
    pub fn new(widget: &impl IsA<gtk::Widget>) -> Self {
        Self {
            widget: widget.clone().upcast(),
            shown: Cell::new(None),
        }
    }

    /// Shows the cursor for `tool` in `color`, rebuilding it only when either changed.
    pub fn show(&self, tool: CurrentDrawingTool, color: Color) {
        if self.shown.get() == Some((tool, color)) {
            return;
        }
        if let Some(cursor) = cursor_for(tool, color) {
            self.widget.set_cursor(Some(&cursor));
            self.shown.set(Some((tool, color)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHAPES: [Shape; 6] = [
        Shape::Pen,
        Shape::Highlighter,
        Shape::Arrow {
            pointing_right: true,
        },
        Shape::Arrow {
            pointing_right: false,
        },
        Shape::Rectangle,
        Shape::Text,
    ];

    fn alpha_at(image: &mut Image, x: i32, y: i32) -> u8 {
        let stride = image.surface.stride() as usize;
        let data = image.surface.data().unwrap();
        // Alpha is the high byte of each native-endian ARGB32 pixel.
        let i = y as usize * stride + x as usize * 4;
        u32::from_ne_bytes(data[i..i + 4].try_into().unwrap()).to_be_bytes()[0]
    }

    #[test]
    fn hotspots_stay_inside_every_size() {
        for shape in SHAPES {
            for size in [24, 32, 43, 64] {
                for scale in [1.0, 1.5, 2.0, 3.0] {
                    let image = render(shape, &Color::RED, size, scale).unwrap();
                    let (hx, hy) = image.hotspot;
                    assert!((0..size).contains(&hx) && (0..size).contains(&hy));
                    assert_eq!(
                        image.surface.width(),
                        (f64::from(size) * scale).ceil() as i32
                    );
                }
            }
        }
    }

    #[test]
    fn nothing_is_clipped_at_the_edges() {
        for shape in SHAPES {
            let mut image = render(shape, &Color::RED, 32, 1.0).unwrap();
            for i in 0..32 {
                for (x, y) in [(i, 0), (i, 31), (0, i), (31, i)] {
                    assert_eq!(alpha_at(&mut image, x, y), 0, "{shape:?} at {x},{y}");
                }
            }
        }
    }

    #[test]
    fn pen_and_highlighter_tips_cover_the_hotspot() {
        for shape in [Shape::Pen, Shape::Highlighter] {
            for scale in [1.0, 2.0] {
                let mut image = render(shape, &Color::BLUE, 32, scale).unwrap();
                let (hx, hy) = image.hotspot;
                let s = scale as i32;
                assert!(
                    alpha_at(&mut image, hx * s, hy * s) > 200,
                    "{shape:?} @{scale}"
                );
            }
        }
    }

    #[test]
    fn crosshair_center_is_open() {
        let mut image = render(Shape::Rectangle, &Color::RED, 32, 1.0).unwrap();
        let (hx, hy) = image.hotspot;
        assert_eq!(alpha_at(&mut image, hx, hy), 0);
        // The arms reach the hotspot's row and column a few pixels out.
        assert!(alpha_at(&mut image, hx + 5, hy) > 200);
        assert!(alpha_at(&mut image, hx, hy - 5) > 200);
    }
}
