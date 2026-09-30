use gtk::cairo;
use gtk::glib;
use gtk::prelude::*;

use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::rc::Rc;

use crate::colors;
use crate::drawing::drawing_tool::CurrentDrawingTool;

const ICON_SIZE: i32 = 18;
const PRESET_SIZE: i32 = 18;
const SWATCH_SIZE: i32 = 24;
const WIDTH_PREVIEW_SIZE: i32 = 22;

const COLOR_PRESETS: [(gtk::gdk::RGBA, &str); 4] = [
    (colors::RED, "Red"),
    (colors::GREEN, "Green"),
    (colors::BLUE, "Blue"),
    (colors::YELLOW, "Yellow"),
];

#[derive(Clone, Copy)]
enum ActionIcon {
    Undo,
    Clear,
    PassThrough,
}

struct ToolButton {
    button: gtk::ToggleButton,
    tool: CurrentDrawingTool,
}

struct PresetButton {
    button: gtk::Button,
    color: gtk::gdk::RGBA,
}

pub struct Toolbar {
    container: gtk::Box,
    tool_buttons: Vec<ToolButton>,
    preset_buttons: Vec<PresetButton>,
    thickness_label: gtk::Label,
    minus_btn: gtk::Button,
    plus_btn: gtk::Button,
    swatch_btn: gtk::Button,
    color_popover: gtk::Popover,
    #[allow(deprecated)]
    color_chooser: gtk::ColorChooserWidget,
    color_swatch: gtk::DrawingArea,
    swatch_color: Rc<RefCell<gtk::gdk::RGBA>>,
    width_preview: gtk::DrawingArea,
    preview_width: Rc<Cell<f64>>,
    undo_btn: gtk::Button,
    clear_btn: gtk::Button,
    pass_through_btn: gtk::ToggleButton,
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

fn set_source(ctx: &cairo::Context, c: &gtk::gdk::RGBA) {
    ctx.set_source_rgba(
        c.red() as f64,
        c.green() as f64,
        c.blue() as f64,
        c.alpha() as f64,
    );
}

fn same_color(a: &gtk::gdk::RGBA, b: &gtk::gdk::RGBA) -> bool {
    let eq = |x: f32, y: f32| (x - y).abs() < 0.002;
    eq(a.red(), b.red()) && eq(a.green(), b.green()) && eq(a.blue(), b.blue())
}

/// Filled circle with a thin light outline so dark colors stay visible on the dark toolbar.
fn draw_color_dot(ctx: &cairo::Context, c: &gtk::gdk::RGBA, w: f64, h: f64) {
    let r = w.min(h) / 2.0 - 1.0;
    ctx.arc(w / 2.0, h / 2.0, r, 0.0, 2.0 * PI);
    set_source(ctx, c);
    let _ = ctx.fill_preserve();
    ctx.set_source_rgba(1.0, 1.0, 1.0, 0.35);
    ctx.set_line_width(1.0);
    let _ = ctx.stroke();
}

/// Adds the arrow icon's path (shaft plus open head) in an `s`-sized box; the tool
/// cursors reuse it for their badge so both show the same glyph.
fn arrow_icon_path(ctx: &cairo::Context, s: f64, pointing_right: bool) {
    let (tail, head) = if pointing_right {
        (0.18 * s, 0.82 * s)
    } else {
        (0.82 * s, 0.18 * s)
    };
    let dir = if pointing_right { -1.0 } else { 1.0 };
    let y = 0.5 * s;
    ctx.move_to(tail, y);
    ctx.line_to(head, y);
    ctx.move_to(head + dir * 0.28 * s, y - 0.24 * s);
    ctx.line_to(head, y);
    ctx.line_to(head + dir * 0.28 * s, y + 0.24 * s);
}

/// Adds the rectangle icon's path in an `s`-sized box, shared with the tool cursors.
pub(crate) fn rectangle_icon_path(ctx: &cairo::Context, s: f64) {
    ctx.rectangle(0.16 * s, 0.26 * s, 0.68 * s, 0.48 * s);
}

/// Draws a monochrome icon for `tool` in the widget's current foreground color.
fn draw_tool_icon(ctx: &cairo::Context, tool: CurrentDrawingTool, fg: &gtk::gdk::RGBA, s: f64) {
    set_source(ctx, fg);
    ctx.set_line_width(1.8);
    ctx.set_line_cap(cairo::LineCap::Round);
    ctx.set_line_join(cairo::LineJoin::Round);

    match tool {
        CurrentDrawingTool::NormalLine => {
            // A freehand squiggle.
            ctx.move_to(0.14 * s, 0.72 * s);
            ctx.curve_to(0.30 * s, 0.20 * s, 0.46 * s, 0.20 * s, 0.52 * s, 0.50 * s);
            ctx.curve_to(0.58 * s, 0.80 * s, 0.74 * s, 0.80 * s, 0.86 * s, 0.28 * s);
            let _ = ctx.stroke();
        }
        CurrentDrawingTool::NormalArrowHeadPointer => {
            arrow_icon_path(ctx, s, true);
            let _ = ctx.stroke();
        }
        CurrentDrawingTool::NormalArrowHeadBase => {
            arrow_icon_path(ctx, s, false);
            let _ = ctx.stroke();
        }
        CurrentDrawingTool::NormalRectangle => {
            rectangle_icon_path(ctx, s);
            let _ = ctx.stroke();
        }
        CurrentDrawingTool::Highlighter => {
            // A translucent marker band with a tilted marker above it.
            ctx.set_source_rgba(
                fg.red() as f64,
                fg.green() as f64,
                fg.blue() as f64,
                fg.alpha() as f64 * 0.5,
            );
            ctx.rectangle(0.10 * s, 0.76 * s, 0.80 * s, 0.14 * s);
            let _ = ctx.fill();

            set_source(ctx, fg);
            ctx.save().ok();
            ctx.translate(0.56 * s, 0.40 * s);
            ctx.rotate(-PI / 4.0);
            // Body.
            ctx.rectangle(-0.10 * s, -0.12 * s, 0.44 * s, 0.24 * s);
            // Chisel tip.
            ctx.move_to(-0.10 * s, -0.12 * s);
            ctx.line_to(-0.26 * s, -0.06 * s);
            ctx.line_to(-0.26 * s, 0.06 * s);
            ctx.line_to(-0.10 * s, 0.12 * s);
            ctx.restore().ok();
            ctx.set_line_width(1.5);
            let _ = ctx.stroke();
        }
        CurrentDrawingTool::TextLabel => {
            ctx.move_to(0.20 * s, 0.22 * s);
            ctx.line_to(0.80 * s, 0.22 * s);
            ctx.move_to(0.50 * s, 0.22 * s);
            ctx.line_to(0.50 * s, 0.82 * s);
            ctx.move_to(0.38 * s, 0.82 * s);
            ctx.line_to(0.62 * s, 0.82 * s);
            let _ = ctx.stroke();
        }
    }
}

fn make_tool_button(tool: CurrentDrawingTool, tooltip: &str) -> gtk::ToggleButton {
    let icon = gtk::DrawingArea::new();
    icon.set_content_width(ICON_SIZE);
    icon.set_content_height(ICON_SIZE);
    icon.set_draw_func(move |area, ctx, w, h| {
        draw_tool_icon(ctx, tool, &area.color(), w.min(h) as f64);
    });

    let btn = gtk::ToggleButton::new();
    btn.set_child(Some(&icon));
    btn.add_css_class("toolbar-tool-btn");
    btn.set_tooltip_text(Some(tooltip));
    // The icon color follows the button state (see style.css), so repaint on toggle.
    btn.connect_toggled(move |_| icon.queue_draw());
    btn
}

fn draw_action_icon(ctx: &cairo::Context, icon: ActionIcon, fg: &gtk::gdk::RGBA, s: f64) {
    set_source(ctx, fg);
    ctx.set_line_width(1.8);
    ctx.set_line_cap(cairo::LineCap::Round);
    ctx.set_line_join(cairo::LineJoin::Round);

    match icon {
        ActionIcon::Undo => {
            // Counter-clockwise hook ending in an arrowhead on the left.
            ctx.move_to(0.24 * s, 0.40 * s);
            ctx.line_to(0.62 * s, 0.40 * s);
            ctx.arc(0.62 * s, 0.58 * s, 0.18 * s, -PI / 2.0, PI / 2.0);
            ctx.line_to(0.34 * s, 0.76 * s);
            ctx.move_to(0.38 * s, 0.24 * s);
            ctx.line_to(0.22 * s, 0.40 * s);
            ctx.line_to(0.38 * s, 0.56 * s);
            let _ = ctx.stroke();
        }
        ActionIcon::Clear => {
            // Trash can: lid, handle, and a tapered bin with two ribs.
            ctx.move_to(0.18 * s, 0.28 * s);
            ctx.line_to(0.82 * s, 0.28 * s);
            ctx.move_to(0.40 * s, 0.28 * s);
            ctx.line_to(0.40 * s, 0.18 * s);
            ctx.line_to(0.60 * s, 0.18 * s);
            ctx.line_to(0.60 * s, 0.28 * s);
            ctx.move_to(0.26 * s, 0.28 * s);
            ctx.line_to(0.31 * s, 0.84 * s);
            ctx.line_to(0.69 * s, 0.84 * s);
            ctx.line_to(0.74 * s, 0.28 * s);
            ctx.move_to(0.44 * s, 0.42 * s);
            ctx.line_to(0.44 * s, 0.70 * s);
            ctx.move_to(0.56 * s, 0.42 * s);
            ctx.line_to(0.56 * s, 0.70 * s);
            let _ = ctx.stroke();
        }
        ActionIcon::PassThrough => {
            // Mouse pointer: clicks go to the desktop underneath.
            ctx.move_to(0.28 * s, 0.14 * s);
            ctx.line_to(0.28 * s, 0.78 * s);
            ctx.line_to(0.44 * s, 0.62 * s);
            ctx.line_to(0.56 * s, 0.86 * s);
            ctx.line_to(0.66 * s, 0.81 * s);
            ctx.line_to(0.54 * s, 0.58 * s);
            ctx.line_to(0.76 * s, 0.58 * s);
            ctx.close_path();
            let _ = ctx.stroke();
        }
    }
}

fn make_action_icon(icon: ActionIcon) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_content_width(ICON_SIZE);
    area.set_content_height(ICON_SIZE);
    area.set_draw_func(move |area, ctx, w, h| {
        draw_action_icon(ctx, icon, &area.color(), w.min(h) as f64);
    });
    area
}

fn make_action_button(icon: ActionIcon, tooltip: &str) -> gtk::Button {
    let area = make_action_icon(icon);
    let btn = gtk::Button::new();
    btn.set_child(Some(&area));
    btn.add_css_class("toolbar-tool-btn");
    btn.set_tooltip_text(Some(tooltip));
    // The icon color follows hover state (see style.css), so repaint on state changes.
    btn.connect_state_flags_changed(glib::clone!(
        #[weak]
        area,
        move |_, _| area.queue_draw()
    ));
    btn
}

fn make_action_toggle(icon: ActionIcon, tooltip: &str) -> gtk::ToggleButton {
    let area = make_action_icon(icon);
    let btn = gtk::ToggleButton::new();
    btn.set_child(Some(&area));
    btn.add_css_class("toolbar-tool-btn");
    btn.set_tooltip_text(Some(tooltip));
    // The icon color follows hover and checked state (see style.css).
    btn.connect_state_flags_changed(glib::clone!(
        #[weak]
        area,
        move |_, _| area.queue_draw()
    ));
    btn
}

fn make_color_button(rgba: gtk::gdk::RGBA) -> gtk::Button {
    let area = gtk::DrawingArea::new();
    area.set_content_width(PRESET_SIZE);
    area.set_content_height(PRESET_SIZE);
    area.set_draw_func(move |_, ctx, w, h| {
        draw_color_dot(ctx, &rgba, w as f64, h as f64);
    });
    let btn = gtk::Button::new();
    btn.set_child(Some(&area));
    btn.add_css_class("toolbar-color-btn");
    btn
}

fn make_group() -> gtk::Box {
    let group = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    group.add_css_class("toolbar-group");
    group.set_valign(gtk::Align::Center);
    group
}

impl Toolbar {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        container.add_css_class("toolbar-palette");
        container.set_margin_top(12);
        container.set_halign(gtk::Align::Center);
        container.set_valign(gtk::Align::Start);

        // Tools
        let tools = [
            (CurrentDrawingTool::NormalLine, "Pen"),
            (CurrentDrawingTool::NormalArrowHeadPointer, "Arrow"),
            (CurrentDrawingTool::NormalArrowHeadBase, "Reverse arrow"),
            (CurrentDrawingTool::NormalRectangle, "Rectangle"),
            (CurrentDrawingTool::Highlighter, "Highlighter"),
            (CurrentDrawingTool::TextLabel, "Text"),
        ];

        let tool_group = make_group();
        let mut tool_buttons: Vec<ToolButton> = Vec::new();
        for (tool, tooltip) in tools {
            let btn = make_tool_button(tool, tooltip);
            if let Some(first) = tool_buttons.first() {
                btn.set_group(Some(&first.button));
            } else {
                btn.set_active(true);
            }
            tool_group.append(&btn);
            tool_buttons.push(ToolButton { button: btn, tool });
        }
        container.append(&tool_group);

        // Colors
        let color_group = make_group();

        let swatch_color = Rc::new(RefCell::new(colors::RED));
        let color_swatch = gtk::DrawingArea::new();
        color_swatch.set_content_width(SWATCH_SIZE);
        color_swatch.set_content_height(SWATCH_SIZE);
        color_swatch.add_css_class("toolbar-swatch");
        color_swatch.set_draw_func(glib::clone!(
            #[strong]
            swatch_color,
            move |_, ctx, w, h| {
                let (w, h) = (w as f64, h as f64);
                // Outer ring hints that this opens the full color chooser.
                ctx.arc(w / 2.0, h / 2.0, w.min(h) / 2.0 - 1.0, 0.0, 2.0 * PI);
                ctx.set_source_rgba(1.0, 1.0, 1.0, 0.85);
                ctx.set_line_width(2.0);
                let _ = ctx.stroke();
                ctx.arc(w / 2.0, h / 2.0, w.min(h) / 2.0 - 4.0, 0.0, 2.0 * PI);
                set_source(ctx, &swatch_color.borrow());
                let _ = ctx.fill();
            },
        ));

        let swatch_btn = gtk::Button::new();
        swatch_btn.set_child(Some(&color_swatch));
        swatch_btn.add_css_class("toolbar-color-btn");
        swatch_btn.add_css_class("toolbar-swatch-btn");
        swatch_btn.set_tooltip_text(Some("Current color (click to choose)"));
        color_group.append(&swatch_btn);

        // The full chooser lives in a popover so it opens as a popup of the overlay
        // instead of a separate toplevel that tiling compositors would tile.
        #[allow(deprecated)]
        let color_chooser = {
            let chooser = gtk::ColorChooserWidget::new();
            chooser.set_use_alpha(true);
            chooser
        };
        let color_popover = gtk::Popover::new();
        color_popover.add_css_class("toolbar-color-popover");
        color_popover.set_child(Some(&color_chooser));
        color_popover.set_position(gtk::PositionType::Bottom);
        color_popover.set_parent(&swatch_btn);

        let sep = gtk::Separator::new(gtk::Orientation::Vertical);
        sep.add_css_class("toolbar-sep");
        color_group.append(&sep);

        let mut preset_buttons = Vec::new();
        for (rgba, tooltip) in COLOR_PRESETS {
            let btn = make_color_button(rgba);
            btn.set_tooltip_text(Some(tooltip));
            color_group.append(&btn);
            preset_buttons.push(PresetButton {
                button: btn,
                color: rgba,
            });
        }
        container.append(&color_group);

        // Line width
        let width_group = make_group();

        let minus_btn = gtk::Button::with_label("−");
        minus_btn.add_css_class("toolbar-tool-btn");
        minus_btn.add_css_class("toolbar-step-btn");
        minus_btn.set_tooltip_text(Some("Decrease line width"));
        width_group.append(&minus_btn);

        let preview_width = Rc::new(Cell::new(5.0_f64));
        let width_preview = gtk::DrawingArea::new();
        width_preview.set_content_width(WIDTH_PREVIEW_SIZE);
        width_preview.set_content_height(WIDTH_PREVIEW_SIZE);
        width_preview.set_valign(gtk::Align::Center);
        width_preview.set_draw_func(glib::clone!(
            #[strong]
            preview_width,
            #[strong]
            swatch_color,
            move |_, ctx, w, h| {
                let (w, h) = (w as f64, h as f64);
                let r = (preview_width.get() / 2.0).clamp(1.0, w.min(h) / 2.0);
                ctx.arc(w / 2.0, h / 2.0, r, 0.0, 2.0 * PI);
                set_source(ctx, &swatch_color.borrow());
                let _ = ctx.fill();
            },
        ));
        width_group.append(&width_preview);

        let thickness_label = gtk::Label::new(Some("5"));
        thickness_label.add_css_class("toolbar-label");
        thickness_label.set_width_chars(2);
        thickness_label.set_xalign(0.5);
        thickness_label.set_tooltip_text(Some("Line width"));
        width_group.append(&thickness_label);

        let plus_btn = gtk::Button::with_label("+");
        plus_btn.add_css_class("toolbar-tool-btn");
        plus_btn.add_css_class("toolbar-step-btn");
        plus_btn.set_tooltip_text(Some("Increase line width"));
        width_group.append(&plus_btn);
        container.append(&width_group);

        // History
        let action_group = make_group();
        let undo_btn = make_action_button(ActionIcon::Undo, "Undo");
        action_group.append(&undo_btn);
        let clear_btn = make_action_button(ActionIcon::Clear, "Clear all");
        clear_btn.add_css_class("toolbar-danger-btn");
        action_group.append(&clear_btn);
        container.append(&action_group);

        let pass_through_btn = make_action_toggle(
            ActionIcon::PassThrough,
            "Use the desktop, keep the drawing (click again to draw)",
        );
        let mode_group = make_group();
        mode_group.append(&pass_through_btn);
        container.append(&mode_group);

        Toolbar {
            container,
            tool_buttons,
            preset_buttons,
            thickness_label,
            minus_btn,
            plus_btn,
            swatch_btn,
            color_popover,
            color_chooser,
            color_swatch,
            swatch_color,
            width_preview,
            preview_width,
            undo_btn,
            clear_btn,
            pass_through_btn,
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }

    pub fn connect_tool_selected<F: Fn(CurrentDrawingTool) + 'static>(&self, f: F) {
        let f = Rc::new(f);
        for tb in &self.tool_buttons {
            let tool = tb.tool;
            let f = f.clone();
            tb.button.connect_toggled(move |btn| {
                if btn.is_active() {
                    f(tool);
                }
            });
        }
    }

    pub fn connect_swatch_clicked<F: Fn() + 'static>(&self, f: F) {
        self.swatch_btn.connect_clicked(move |_| f());
    }

    /// Called with each color picked in the color chooser popover.
    pub fn connect_color_chosen<F: Fn(gtk::gdk::RGBA) + 'static>(&self, f: F) {
        #[allow(deprecated)]
        self.color_chooser
            .connect_rgba_notify(move |chooser| f(chooser.rgba()));
    }

    pub fn open_color_chooser(&self, current: &gtk::gdk::RGBA) {
        #[allow(deprecated)]
        self.color_chooser.set_rgba(current);
        self.color_popover.popup();
    }

    pub fn color_chooser_open(&self) -> bool {
        self.color_popover.is_visible()
    }

    pub fn connect_preset_selected<F: Fn(gtk::gdk::RGBA) + 'static>(&self, f: F) {
        let f = Rc::new(f);
        for pb in &self.preset_buttons {
            let color = pb.color;
            let f = f.clone();
            pb.button.connect_clicked(move |_| f(color));
        }
    }

    pub fn connect_line_width_changed<F: Fn(f64) + 'static>(&self, f: F) {
        let f = Rc::new(f);
        let f_plus = f.clone();
        self.plus_btn.connect_clicked(move |_| f_plus(1.0));
        let f_minus = f.clone();
        self.minus_btn.connect_clicked(move |_| f_minus(-1.0));
    }

    pub fn connect_undo<F: Fn() + 'static>(&self, f: F) {
        self.undo_btn.connect_clicked(move |_| f());
    }

    pub fn connect_clear<F: Fn() + 'static>(&self, f: F) {
        self.clear_btn.connect_clicked(move |_| f());
    }

    /// Called with the new state when the pass-through toggle is clicked.
    pub fn connect_pass_through<F: Fn(bool) + 'static>(&self, f: F) {
        self.pass_through_btn
            .connect_clicked(move |btn| f(btn.is_active()));
    }

    /// Shows the pass-through state without emitting the click handler.
    pub fn set_pass_through(&self, on: bool) {
        self.pass_through_btn.set_active(on);
        if on {
            self.color_popover.popdown();
        }
    }

    pub fn pass_through(&self) -> bool {
        self.pass_through_btn.is_active()
    }

    pub fn set_active_tool(&self, tool: CurrentDrawingTool) {
        for tb in &self.tool_buttons {
            if tb.tool == tool {
                tb.button.set_active(true);
                break;
            }
        }
    }

    pub fn update(&self, tool: &CurrentDrawingTool, color: &gtk::gdk::RGBA, line_width: f64) {
        self.set_active_tool(*tool);
        self.thickness_label.set_text(&format!("{:.0}", line_width));
        *self.swatch_color.borrow_mut() = *color;
        self.preview_width.set(line_width);
        for pb in &self.preset_buttons {
            if same_color(&pb.color, color) {
                pb.button.add_css_class("selected");
            } else {
                pb.button.remove_css_class("selected");
            }
        }
        self.color_swatch.queue_draw();
        self.width_preview.queue_draw();
    }
}
