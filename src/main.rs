use drawing::drawing_tool::DrawingTool;

use gtk::glib::{self, Propagation};
use gtk::{
    cairo::Region,
    gdk::{Display, Key},
    gio,
    prelude::*,
};
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};

use std::{cell::RefCell, rc::Rc};

pub mod colors;
pub mod config;
pub mod cursors;
pub mod drawing;
pub mod geometry;
pub mod toolbar;

// https://github.com/wmww/gtk-layer-shell/blob/master/examples/simple-example.c
fn activate(application: &gtk::Application) {
    // Create a normal GTK window however you like
    let window = gtk::ApplicationWindow::new(application);

    application.connect_activate(glib::clone!(
        #[weak]
        window,
        move |_| {
            window.set_keyboard_mode(KeyboardMode::Exclusive);
            if let Some(surface) = window.surface() {
                surface.set_opaque_region(Some(&Region::create()));
            }
        },
    ));

    let conf = Rc::new(config::get_config());

    // Before the window is first realized, set it up to be a layer surface
    window.init_layer_shell();
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    // Display above normal windows
    window.set_layer(Layer::Overlay);
    // Anchors are if the window is pinned to each edge of the output
    let anchors = [
        (gtk4_layer_shell::Edge::Left, true),
        (gtk4_layer_shell::Edge::Right, true),
        (gtk4_layer_shell::Edge::Top, true),
        (gtk4_layer_shell::Edge::Bottom, true),
    ];

    for (anchor, state) in anchors {
        window.set_anchor(anchor, state);
    }

    {
        let display = Display::default().expect("error getting default display");
        let target_monitor = display
            .default_seat()
            .and_then(|s| s.pointer())
            .and_then(|pointer| {
                let surface = pointer.surface_at_position();
                surface.0.and_then(|s| display.monitor_at_surface(&s))
            });
        if let Some(monitor) = target_monitor {
            window.set_monitor(Some(&monitor));
        }
    }

    // main components
    let elements: Rc<RefCell<Vec<Box<dyn DrawingTool>>>> = Rc::new(RefCell::new(Vec::new()));

    let color = Rc::new(RefCell::new(colors::RED));

    let current_tool = Rc::new(RefCell::new(
        drawing::drawing_tool::CurrentDrawingTool::NormalLine,
    ));

    let text_input_mode = Rc::new(RefCell::new(false));
    let shift_held = Rc::new(RefCell::new(false));

    let key_controller = gtk::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);

    let color_dialog = Rc::new(
        gtk::ColorDialog::builder()
            .title("Choose color")
            .modal(true)
            .build(),
    );

    // generate tool cursors at runtime using Cairo
    let pencil_cur = cursors::pencil_cursor();
    let arrow_cur = cursors::arrow_cursor();
    let rectangle_cur = cursors::rectangle_cursor();
    let text_cur = cursors::text_cursor();
    let highlighter_cur = cursors::highlighter_cursor();

    // Set up a widget
    let draw = gtk::DrawingArea::new();
    draw.set_focusable(true);
    // the default cursor should be the pencil one
    if let Some(pencil_cur) = pencil_cur.clone() {
        draw.set_cursor(Some(&pencil_cur));
    }

    let line_width = Rc::new(RefCell::new(conf.line_thickness.unwrap_or(2.0)));

    let toolbar = Rc::new(RefCell::new(toolbar::Toolbar::new()));
    toolbar.borrow().update(
        &drawing::drawing_tool::CurrentDrawingTool::NormalLine,
        &colors::RED,
        conf.line_thickness.unwrap_or(2.0),
    );

    toolbar.borrow().connect_tool_selected(glib::clone!(
        #[strong]
        current_tool,
        #[strong]
        draw,
        #[strong]
        pencil_cur,
        #[strong]
        arrow_cur,
        #[strong]
        rectangle_cur,
        #[strong]
        text_cur,
        #[strong]
        highlighter_cur,
        move |tool| {
            *current_tool.borrow_mut() = tool;
            let cursor = match tool {
                drawing::drawing_tool::CurrentDrawingTool::NormalLine => pencil_cur.clone(),
                drawing::drawing_tool::CurrentDrawingTool::NormalArrowHeadBase
                | drawing::drawing_tool::CurrentDrawingTool::NormalArrowHeadPointer => {
                    arrow_cur.clone()
                }
                drawing::drawing_tool::CurrentDrawingTool::NormalRectangle => rectangle_cur.clone(),
                drawing::drawing_tool::CurrentDrawingTool::TextLabel => text_cur.clone(),
                drawing::drawing_tool::CurrentDrawingTool::Highlighter => highlighter_cur.clone(),
            };
            if let Some(c) = cursor {
                draw.set_cursor(Some(&c));
            }
        },
    ));

    toolbar.borrow().connect_swatch_clicked(glib::clone!(
        #[strong(rename_to = w)]
        window,
        #[strong]
        color_dialog,
        #[strong]
        color,
        #[strong]
        toolbar,
        #[strong]
        current_tool,
        #[strong]
        line_width,
        move || {
            w.set_layer(Layer::Bottom);
            color_dialog.choose_rgba(
                None::<&gtk::Window>,
                Some(&*color.borrow()),
                None::<&gio::Cancellable>,
                glib::clone!(
                    #[strong]
                    color,
                    #[strong]
                    toolbar,
                    #[strong]
                    current_tool,
                    #[strong]
                    line_width,
                    #[weak]
                    w,
                    move |c| match c {
                        Ok(c) => {
                            w.set_layer(Layer::Overlay);
                            *color.borrow_mut() = c;
                            toolbar.borrow().update(
                                &current_tool.borrow(),
                                &c,
                                *line_width.borrow(),
                            );
                        }
                        Err(_) => {
                            w.set_layer(Layer::Overlay);
                        }
                    },
                ),
            );
        },
    ));

    toolbar.borrow().connect_preset_selected(glib::clone!(
        #[strong]
        color,
        #[strong]
        toolbar,
        #[strong]
        current_tool,
        #[strong]
        line_width,
        move |rgba| {
            *color.borrow_mut() = rgba;
            toolbar
                .borrow()
                .update(&current_tool.borrow(), &rgba, *line_width.borrow());
        },
    ));

    toolbar.borrow().connect_line_width_changed(glib::clone!(
        #[strong]
        line_width,
        #[strong]
        toolbar,
        #[strong]
        current_tool,
        #[strong]
        color,
        move |delta| {
            let mut width = line_width.borrow_mut();
            let new_width = *width + delta;
            *width = if new_width < 1.0 { 1.0 } else { new_width };
            toolbar
                .borrow()
                .update(&current_tool.borrow(), &color.borrow(), *width);
        },
    ));

    key_controller.connect_key_pressed(glib::clone!(
        #[strong]
        draw,
        #[strong(rename_to = w)]
        window,
        #[strong]
        color_dialog,
        #[strong]
        conf,
        #[strong]
        color,
        #[strong]
        current_tool,
        #[strong]
        text_input_mode,
        #[strong]
        elements,
        #[strong]
        toolbar,
        #[strong]
        line_width,
        #[strong]
        shift_held,
        move |_, keyval, _, modifier| {
            let is_shift = modifier.contains(gtk::gdk::ModifierType::SHIFT_MASK);
            *shift_held.borrow_mut() = is_shift;
            if let Some(elem) = elements.borrow_mut().last_mut() {
                if elem.active() {
                    elem.set_constrained(is_shift);
                }
            }
            draw.queue_draw();
            if *text_input_mode.borrow() {
                let _draw_key = Key::from_name(conf.draw_keybind.as_deref().unwrap_or(""))
                    .unwrap_or(Key::Abelowdot);
                let _arrow_key = Key::from_name(conf.arrow_keybind.as_deref().unwrap_or(""))
                    .unwrap_or(Key::Abelowdot);
                let _reverse_arrow_key =
                    Key::from_name(conf.reverse_arrow_keybind.as_deref().unwrap_or(""))
                        .unwrap_or(Key::Abelowdot);
                let _rectangle_key =
                    Key::from_name(conf.rectangle_keybind.as_deref().unwrap_or(""))
                        .unwrap_or(Key::Abelowdot);
                let _text_key = Key::from_name(conf.text_keybind.as_deref().unwrap_or(""))
                    .unwrap_or(Key::Abelowdot);
                let _highlighter_key =
                    Key::from_name(conf.highlighter_keybind.as_deref().unwrap_or(""))
                        .unwrap_or(Key::Abelowdot);

                let is_tool_switch = keyval == _draw_key
                    || keyval == _arrow_key
                    || keyval == _reverse_arrow_key
                    || keyval == _rectangle_key
                    || keyval == _text_key
                    || keyval == _highlighter_key;

                if is_tool_switch || keyval == Key::Escape {
                    if let Some(elem) = elements.borrow_mut().last_mut() {
                        if let Some(text_tool) = elem
                            .as_any_mut()
                            .downcast_mut::<drawing::text_label::TextLabel>()
                        {
                            text_tool.finish();
                        }
                    }
                    *text_input_mode.borrow_mut() = false;
                    draw.queue_draw();
                    if keyval == Key::Escape {
                        return Propagation::Stop;
                    }
                } else {
                    match keyval {
                        Key::Return => {
                            if let Some(elem) = elements.borrow_mut().last_mut() {
                                if let Some(text_tool) =
                                    elem.as_any_mut()
                                        .downcast_mut::<drawing::text_label::TextLabel>()
                                {
                                    text_tool.finish();
                                }
                            }
                            *text_input_mode.borrow_mut() = false;
                        }
                        Key::BackSpace => {
                            if let Some(elem) = elements.borrow_mut().last_mut() {
                                if let Some(text_tool) =
                                    elem.as_any_mut()
                                        .downcast_mut::<drawing::text_label::TextLabel>()
                                {
                                    text_tool.pop_char();
                                }
                            }
                            draw.queue_draw();
                        }
                        _ => {
                            if let Some(c) = keyval.to_unicode() {
                                if !c.is_control() {
                                    if let Some(elem) = elements.borrow_mut().last_mut() {
                                        if let Some(text_tool) =
                                            elem.as_any_mut()
                                                .downcast_mut::<drawing::text_label::TextLabel>()
                                        {
                                            text_tool.push_char(c);
                                        }
                                    }
                                    draw.queue_draw();
                                }
                            }
                        }
                    }
                    return Propagation::Stop;
                }
            }

            // close your eyes
            let _draw_key = Key::from_name(conf.draw_keybind.as_deref().unwrap_or(""))
                .unwrap_or(Key::Abelowdot);
            let _arrow_key = Key::from_name(conf.arrow_keybind.as_deref().unwrap_or(""))
                .unwrap_or(Key::Abelowdot);
            let _reverse_arrow_key =
                Key::from_name(conf.reverse_arrow_keybind.as_deref().unwrap_or(""))
                    .unwrap_or(Key::Abelowdot);
            let _rectangle_key = Key::from_name(conf.rectangle_keybind.as_deref().unwrap_or(""))
                .unwrap_or(Key::Abelowdot);
            let _text_key = Key::from_name(conf.text_keybind.as_deref().unwrap_or(""))
                .unwrap_or(Key::Abelowdot);
            let _highlighter_key =
                Key::from_name(conf.highlighter_keybind.as_deref().unwrap_or(""))
                    .unwrap_or(Key::Abelowdot);
            let _disable_drawing_key =
                Key::from_name(conf.disable_drawing.as_deref().unwrap_or(""))
                    .unwrap_or(Key::Abelowdot);
            let _color_r =
                Key::from_name(conf.color_r.as_deref().unwrap_or("")).unwrap_or(Key::Abelowdot);
            let _color_g =
                Key::from_name(conf.color_g.as_deref().unwrap_or("")).unwrap_or(Key::Abelowdot);
            let _color_b =
                Key::from_name(conf.color_b.as_deref().unwrap_or("")).unwrap_or(Key::Abelowdot);
            let _color_chooser = Key::from_name(conf.color_chooser.as_deref().unwrap_or(""))
                .unwrap_or(Key::Abelowdot);
            let _undo_key =
                Key::from_name(conf.undo.as_deref().unwrap_or("")).unwrap_or(Key::Abelowdot);
            let _clear_all_key =
                Key::from_name(conf.clear_all.as_deref().unwrap_or("")).unwrap_or(Key::Abelowdot);

            match keyval {
                // TOOLS
                _ if _draw_key == keyval => {
                    *current_tool.borrow_mut() =
                        drawing::drawing_tool::CurrentDrawingTool::NormalLine;
                    if let Some(pencil_cur) = pencil_cur.clone() {
                        draw.set_cursor(Some(&pencil_cur));
                    }
                }
                _ if _arrow_key == keyval => {
                    *current_tool.borrow_mut() =
                        drawing::drawing_tool::CurrentDrawingTool::NormalArrowHeadPointer;
                    if let Some(arrow_cur) = arrow_cur.clone() {
                        draw.set_cursor(Some(&arrow_cur));
                    }
                }
                _ if _reverse_arrow_key == keyval => {
                    *current_tool.borrow_mut() =
                        drawing::drawing_tool::CurrentDrawingTool::NormalArrowHeadBase;
                    if let Some(arrow_cur) = arrow_cur.clone() {
                        draw.set_cursor(Some(&arrow_cur));
                    }
                }
                _ if _rectangle_key == keyval => {
                    *current_tool.borrow_mut() =
                        drawing::drawing_tool::CurrentDrawingTool::NormalRectangle;
                    if let Some(rectangle_cur) = rectangle_cur.clone() {
                        draw.set_cursor(Some(&rectangle_cur));
                    }
                }
                _ if _text_key == keyval => {
                    *current_tool.borrow_mut() =
                        drawing::drawing_tool::CurrentDrawingTool::TextLabel;
                    if let Some(text_cur) = text_cur.clone() {
                        draw.set_cursor(Some(&text_cur));
                    }
                }
                _ if _highlighter_key == keyval => {
                    *current_tool.borrow_mut() =
                        drawing::drawing_tool::CurrentDrawingTool::Highlighter;
                    if let Some(highlighter_cur) = highlighter_cur.clone() {
                        draw.set_cursor(Some(&highlighter_cur));
                    }
                }
                _ if _disable_drawing_key == keyval => {
                    w.set_keyboard_mode(KeyboardMode::None);
                    if let Some(surface) = w.surface() {
                        surface.set_input_region(&Region::create());
                    }
                    w.unmap();
                    w.map();
                }
                // colors
                _ if _color_r == keyval => *color.borrow_mut() = colors::RED,
                _ if _color_g == keyval => *color.borrow_mut() = colors::GREEN,
                _ if _color_b == keyval => *color.borrow_mut() = colors::BLUE,
                _ if _undo_key == keyval
                    && modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK) =>
                {
                    elements.borrow_mut().pop();
                    draw.queue_draw();
                }
                _ if _clear_all_key == keyval
                    && modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK) =>
                {
                    elements.borrow_mut().clear();
                    draw.queue_draw();
                }
                _ if _color_chooser == keyval => {
                    w.set_layer(Layer::Bottom);
                    color_dialog.choose_rgba(
                        None::<&gtk::Window>,
                        Some(&*color.borrow()),
                        None::<&gio::Cancellable>,
                        glib::clone!(
                            #[strong]
                            color,
                            #[strong]
                            toolbar,
                            #[strong]
                            current_tool,
                            #[strong]
                            line_width,
                            #[weak]
                            w,
                            move |c| match c {
                                Ok(c) => {
                                    w.set_layer(Layer::Overlay);
                                    *color.borrow_mut() = c;
                                    toolbar.borrow().update(
                                        &current_tool.borrow(),
                                        &c,
                                        *line_width.borrow(),
                                    );
                                }
                                Err(_) => {
                                    w.set_layer(Layer::Overlay);
                                }
                            },
                        ),
                    );
                }
                _ => (),
            };
            toolbar.borrow().update(
                &current_tool.borrow(),
                &color.borrow(),
                *line_width.borrow(),
            );
            Propagation::Proceed
        },
    ));

    key_controller.connect_key_released(glib::clone!(
        #[strong]
        shift_held,
        #[strong]
        elements,
        #[weak]
        draw,
        move |_, _, _, modifier| {
            let is_shift = modifier.contains(gtk::gdk::ModifierType::SHIFT_MASK);
            *shift_held.borrow_mut() = is_shift;
            if let Some(elem) = elements.borrow_mut().last_mut() {
                if elem.active() {
                    elem.set_constrained(is_shift);
                }
            }
            draw.queue_draw();
        },
    ));

    // key controller is added to the window and not to the drawarea because there it does not
    // work
    window.add_controller(key_controller);

    let motion_controller = gtk::EventControllerMotion::new();
    motion_controller.connect_motion(glib::clone!(
        #[weak]
        draw,
        #[strong]
        elements,
        move |_, x, y| {
            if let Some(elem) = elements.borrow_mut().last_mut() {
                elem.motion_notify(drawing::drawing_tool::Point(x, y));
                if elem.active() {
                    draw.queue_draw();
                }
            }
        },
    ));

    draw.add_controller(motion_controller);

    let right_click_mouse = gtk::GestureClick::new();

    // Set the gestures button to the right mouse button (=3)
    right_click_mouse.set_button(gtk::gdk::ffi::GDK_BUTTON_SECONDARY as u32);

    // Assign your handler to an event of the gesture (e.g. the `pressed` event)
    right_click_mouse.connect_pressed(|_, _, _, _| {
        // exit the application
        std::process::exit(0);
    });

    draw.add_controller(right_click_mouse);

    let left_click_mouse = gtk::GestureClick::new();

    // Set the gestures button to the right mouse button (=3)
    left_click_mouse.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);

    // Assign your handler to an event of the gesture (e.g. the `pressed` event)
    left_click_mouse.connect_pressed(glib::clone!(
        #[strong]
        elements,
        #[strong]
        current_tool,
        #[strong]
        line_width,
        #[strong]
        text_input_mode,
        #[strong]
        color,
        #[weak]
        draw,
        move |_, _, x, y| {
            {
                let mut elems = elements.borrow_mut();
                if let Some(elem) = elems.last_mut() {
                    if let Some(text_tool) = elem
                        .as_any_mut()
                        .downcast_mut::<drawing::text_label::TextLabel>()
                    {
                        if text_tool.is_movable() {
                            text_tool.finalize();
                            draw.queue_draw();
                            return;
                        }
                    }
                }
            }
            let mut drawing_tool: Box<dyn drawing::drawing_tool::DrawingTool> =
                match *current_tool.borrow() {
                    drawing::drawing_tool::CurrentDrawingTool::NormalLine => {
                        Box::new(drawing::normal_line::NormalLine::new())
                    }
                    drawing::drawing_tool::CurrentDrawingTool::NormalArrowHeadBase => {
                        Box::new(drawing::arrow::NormalArrow::new(true))
                    }
                    drawing::drawing_tool::CurrentDrawingTool::NormalArrowHeadPointer => {
                        Box::new(drawing::arrow::NormalArrow::new(false))
                    }
                    drawing::drawing_tool::CurrentDrawingTool::NormalRectangle => {
                        Box::new(drawing::normal_rectangle::NormalRectangle::new())
                    }
                    drawing::drawing_tool::CurrentDrawingTool::Highlighter => {
                        Box::new(drawing::highlighter::Highlighter::new())
                    }
                    drawing::drawing_tool::CurrentDrawingTool::TextLabel => {
                        *text_input_mode.borrow_mut() = true;
                        draw.grab_focus();
                        Box::new(drawing::text_label::TextLabel::new())
                    }
                };
            drawing_tool.press_mouse(drawing::drawing_tool::Point(x, y));
            if !matches!(
                *current_tool.borrow(),
                drawing::drawing_tool::CurrentDrawingTool::Highlighter
            ) {
                drawing_tool.set_line_width(*line_width.borrow());
                drawing_tool.set_color(*color.borrow());
            }
            elements.borrow_mut().push(drawing_tool);
        },
    ));

    left_click_mouse.connect_released(glib::clone!(
        #[strong]
        elements,
        move |_, _, x, y| {
            if let Some(elem) = elements.borrow_mut().last_mut() {
                elem.release_mouse(drawing::drawing_tool::Point(x, y));
            }
        },
    ));

    draw.add_controller(left_click_mouse);

    // scroll controller
    let scroll_controller =
        gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);

    scroll_controller.connect_scroll(glib::clone!(
        #[strong]
        line_width,
        #[strong]
        text_input_mode,
        #[strong]
        elements,
        #[strong]
        toolbar,
        #[strong]
        current_tool,
        #[strong]
        color,
        #[weak]
        draw,
        #[upgrade_or]
        Propagation::Proceed,
        move |_, _, scroll| {
            let mut width = line_width.borrow_mut();
            let new_width = *width - scroll;
            if new_width as i32 >= 1 {
                *width = new_width;
            } else {
                *width = 1.0;
            }
            if *text_input_mode.borrow() {
                if let Some(elem) = elements.borrow_mut().last_mut() {
                    elem.set_line_width(*width);
                }
                draw.queue_draw();
            }
            toolbar
                .borrow()
                .update(&current_tool.borrow(), &color.borrow(), *width);
            Propagation::Proceed
        },
    ));

    draw.add_controller(scroll_controller);

    draw.set_draw_func(glib::clone!(
        #[weak]
        elements,
        move |_, ctx, _, _| {
            for element in elements.borrow_mut().iter() {
                element.draw(ctx);
            }

            if let Err(error) = ctx.fill() {
                panic!("error drawing: {:?}", error)
            };
        },
    ));

    // load css for the transparency of the window
    let provider = gtk::CssProvider::new();
    provider.load_from_data(include_str!("styles/style.css"));
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("error getting default display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&draw));
    overlay.add_overlay(toolbar.borrow().widget());

    window.set_child(Some(&overlay));
    window.set_visible(true);
}

fn main() {
    let application = gtk::Application::new(Some("sh.wmww.gtk-layer-example"), Default::default());

    application.connect_activate(|app| {
        activate(app);
    });

    application.run();
}
