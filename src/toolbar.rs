use gtk::glib;
use gtk::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

use crate::colors;
use crate::drawing::drawing_tool::CurrentDrawingTool;

struct ToolButton {
    button: gtk::ToggleButton,
    tool: CurrentDrawingTool,
}

pub struct Toolbar {
    container: gtk::Box,
    tool_buttons: Vec<ToolButton>,
    thickness_label: gtk::Label,
    minus_btn: gtk::Button,
    plus_btn: gtk::Button,
    color_swatch: gtk::DrawingArea,
    swatch_color: Rc<RefCell<gtk::gdk::RGBA>>,
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

fn make_color_button(rgba: gtk::gdk::RGBA) -> gtk::Button {
    let area = gtk::DrawingArea::new();
    area.set_content_width(16);
    area.set_content_height(16);
    area.set_draw_func(move |_, ctx, w, h| {
        ctx.set_source_rgba(
            rgba.red() as f64,
            rgba.green() as f64,
            rgba.blue() as f64,
            rgba.alpha() as f64,
        );
        ctx.rectangle(0.0, 0.0, w as f64, h as f64);
        let _ = ctx.fill();
        ctx.set_source_rgb(1.0, 1.0, 1.0);
        ctx.set_line_width(1.0);
        ctx.rectangle(0.5, 0.5, w as f64 - 1.0, h as f64 - 1.0);
        let _ = ctx.stroke();
    });
    let btn = gtk::Button::new();
    btn.set_child(Some(&area));
    btn.add_css_class("toolbar-color-btn");
    btn
}

impl Toolbar {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        container.add_css_class("toolbar-palette");
        container.set_margin_start(12);
        container.set_margin_top(12);
        container.set_halign(gtk::Align::Start);
        container.set_valign(gtk::Align::Start);

        let tools = [
            (CurrentDrawingTool::NormalLine, "Pen"),
            (CurrentDrawingTool::NormalArrowHeadPointer, "→"),
            (CurrentDrawingTool::NormalArrowHeadBase, "←"),
            (CurrentDrawingTool::NormalRectangle, "▭"),
            (CurrentDrawingTool::Highlighter, "HL"),
            (CurrentDrawingTool::TextLabel, "T"),
        ];

        let mut tool_buttons = Vec::new();

        for (i, (tool, label)) in tools.iter().enumerate() {
            let btn = gtk::ToggleButton::with_label(label);
            btn.add_css_class("toolbar-tool-btn");
            if i == 0 {
                btn.set_active(true);
            }
            container.append(&btn);
            tool_buttons.push(ToolButton {
                button: btn,
                tool: *tool,
            });
        }

        if let Some(first) = tool_buttons.first() {
            for tb in tool_buttons.iter().skip(1) {
                tb.button.set_group(Some(&first.button));
            }
        }

        let sep1 = gtk::Separator::new(gtk::Orientation::Vertical);
        sep1.add_css_class("toolbar-sep");
        container.append(&sep1);

        let swatch_color = Rc::new(RefCell::new(gtk::gdk::RGBA::RED));
        let color_swatch = gtk::DrawingArea::new();
        color_swatch.set_content_width(20);
        color_swatch.set_content_height(20);
        color_swatch.add_css_class("toolbar-swatch");

        color_swatch.set_draw_func(glib::clone!(
            #[strong]
            swatch_color,
            move |_, ctx, width, height| {
                let c = *swatch_color.borrow();
                ctx.set_source_rgba(
                    c.red() as f64,
                    c.green() as f64,
                    c.blue() as f64,
                    c.alpha() as f64,
                );
                ctx.rectangle(0.0, 0.0, width as f64, height as f64);
                let _ = ctx.fill();

                ctx.set_source_rgb(1.0, 1.0, 1.0);
                ctx.set_line_width(1.0);
                ctx.rectangle(0.5, 0.5, width as f64 - 1.0, height as f64 - 1.0);
                let _ = ctx.stroke();
            },
        ));

        let swatch_btn = gtk::Button::new();
        swatch_btn.set_child(Some(&color_swatch));
        swatch_btn.add_css_class("toolbar-color-btn");
        swatch_btn.set_tooltip_text(Some("Color chooser"));
        container.append(&swatch_btn);

        let color_presets = [
            (colors::RED, "Red"),
            (colors::GREEN, "Green"),
            (colors::BLUE, "Blue"),
            (colors::YELLOW, "Yellow"),
        ];

        for (rgba, tooltip) in &color_presets {
            let btn = make_color_button(*rgba);
            btn.set_tooltip_text(Some(tooltip));
            container.append(&btn);
        }

        let sep2 = gtk::Separator::new(gtk::Orientation::Vertical);
        sep2.add_css_class("toolbar-sep");
        container.append(&sep2);

        let minus_btn = gtk::Button::with_label("−");
        minus_btn.add_css_class("toolbar-tool-btn");
        minus_btn.set_tooltip_text(Some("Decrease line width"));
        container.append(&minus_btn);

        let thickness_label = gtk::Label::new(Some("5"));
        thickness_label.add_css_class("toolbar-label");
        thickness_label.set_width_chars(2);
        container.append(&thickness_label);

        let plus_btn = gtk::Button::with_label("+");
        plus_btn.add_css_class("toolbar-tool-btn");
        plus_btn.set_tooltip_text(Some("Increase line width"));
        container.append(&plus_btn);

        Toolbar {
            container,
            tool_buttons,
            thickness_label,
            minus_btn,
            plus_btn,
            color_swatch,
            swatch_color,
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
        let swatch_btn = self
            .color_swatch
            .parent()
            .unwrap()
            .downcast::<gtk::Button>()
            .unwrap();
        swatch_btn.connect_clicked(move |_| f());
    }

    pub fn connect_preset_selected<F: Fn(gtk::gdk::RGBA) + 'static>(&self, f: F) {
        let f = Rc::new(f);
        let presets = [colors::RED, colors::GREEN, colors::BLUE, colors::YELLOW];

        // Children order: [tool_buttons...] [sep1] [swatch_btn] [preset_btns...] [sep2] [thickness]
        // Preset buttons start at index: tool_buttons.len() + 1 (sep) + 1 (swatch) = len+2.
        let sep1_idx = self.tool_buttons.len();
        let start = sep1_idx + 2;

        let container_child = self.container.first_child();
        let mut children = Vec::new();
        let mut child = container_child;
        while let Some(c) = child {
            children.push(c.clone());
            child = c.next_sibling();
        }

        for (i, rgba) in presets.iter().enumerate() {
            let idx = start + i;
            if let Some(widget) = children.get(idx) {
                if let Ok(btn) = widget.clone().downcast::<gtk::Button>() {
                    let color = *rgba;
                    let f = f.clone();
                    btn.connect_clicked(move |_| f(color));
                }
            }
        }
    }

    pub fn connect_line_width_changed<F: Fn(f64) + 'static>(&self, f: F) {
        let f = Rc::new(f);
        let f_plus = f.clone();
        self.plus_btn.connect_clicked(move |_| f_plus(1.0));
        let f_minus = f.clone();
        self.minus_btn.connect_clicked(move |_| f_minus(-1.0));
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
        self.color_swatch.queue_draw();
    }
}
