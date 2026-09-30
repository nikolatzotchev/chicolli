use drawing::drawing_tool::DrawingTool;

use gtk::glib::{self, Propagation};
use gtk::{
    cairo::{RectangleInt, Region},
    gdk::{Display, Key},
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

type Elements = Rc<RefCell<Vec<Box<dyn DrawingTool>>>>;

/// Ends typing into the text label being edited, if any, leaving it where it is.
/// A label that ended up with no text is removed so Undo never pops something invisible.
fn end_text_input(elements: &Elements, text_input_mode: &RefCell<bool>) {
    *text_input_mode.borrow_mut() = false;
    let mut elems = elements.borrow_mut();
    let Some(elem) = elems.last_mut() else {
        return;
    };
    let Some(label) = elem
        .as_any_mut()
        .downcast_mut::<drawing::text_label::TextLabel>()
    else {
        return;
    };
    if !label.is_editing() {
        return;
    }
    if label.is_empty() {
        elems.pop();
    } else {
        label.commit();
    }
}

/// Runs `f` on the text label being typed into, if there is one.
fn with_editing_label<R>(
    elements: &Elements,
    f: impl FnOnce(&mut drawing::text_label::TextLabel) -> R,
) -> Option<R> {
    let mut elems = elements.borrow_mut();
    let label = elems
        .last_mut()?
        .as_any_mut()
        .downcast_mut::<drawing::text_label::TextLabel>()?;
    label.is_editing().then(|| f(label))
}

/// Whether Shift is down after this key event. The event's modifier state is the one
/// from before the key changed, so a Shift key's own press/release has to be applied.
fn shift_after_key_event(keyval: Key, modifier: gtk::gdk::ModifierType, pressed: bool) -> bool {
    match keyval {
        Key::Shift_L | Key::Shift_R => pressed,
        _ => modifier.contains(gtk::gdk::ModifierType::SHIFT_MASK),
    }
}

/// Applies the Shift constraint to the element currently being drawn.
fn constrain_active(elements: &Elements, shift: bool) {
    if let Some(elem) = elements.borrow_mut().last_mut() {
        if elem.active() {
            elem.set_constrained(shift);
        }
    }
}

/// Switches between drawing and pass-through. In pass-through the drawing stays on screen
/// while clicks and keys go to the windows underneath; only the toolbar still takes clicks,
/// so its pass-through toggle can switch back.
fn set_pass_through(window: &gtk::ApplicationWindow, toolbar: &toolbar::Toolbar, on: bool) {
    toolbar.set_pass_through(on);
    window.set_keyboard_mode(if on {
        KeyboardMode::None
    } else {
        KeyboardMode::Exclusive
    });
    if let Some(surface) = window.surface() {
        let region = if on {
            match toolbar.widget().compute_bounds(window) {
                Some(b) => Region::create_rectangle(&RectangleInt::new(
                    b.x().floor() as i32,
                    b.y().floor() as i32,
                    b.width().ceil() as i32,
                    b.height().ceil() as i32,
                )),
                None => Region::create(),
            }
        } else {
            Region::create_rectangle(&RectangleInt::new(0, 0, surface.width(), surface.height()))
        };
        surface.set_input_region(&region);
    }
    // Remap so the compositor picks up the new keyboard mode right away.
    window.unmap();
    window.map();
}

// https://github.com/wmww/gtk-layer-shell/blob/master/examples/simple-example.c
fn activate(application: &gtk::Application) {
    // Create a normal GTK window however you like
    let window = gtk::ApplicationWindow::new(application);

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
    let elements: Elements = Rc::new(RefCell::new(Vec::new()));

    let color = Rc::new(RefCell::new(colors::RED));

    let current_tool = Rc::new(RefCell::new(
        drawing::drawing_tool::CurrentDrawingTool::NormalLine,
    ));

    let text_input_mode = Rc::new(RefCell::new(false));
    let shift_held = Rc::new(RefCell::new(false));

    let key_controller = gtk::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);

    // Set up a widget
    let draw = gtk::DrawingArea::new();
    draw.set_focusable(true);
    let cursor = Rc::new(cursors::ToolCursor::new(&draw));
    cursor.show(*current_tool.borrow(), *color.borrow());

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
        cursor,
        #[strong]
        color,
        #[strong]
        elements,
        #[strong]
        text_input_mode,
        #[weak]
        window,
        #[strong]
        toolbar,
        move |tool| {
            // Picking a tool while passing clicks through means the user wants to draw again.
            if toolbar.borrow().pass_through() {
                set_pass_through(&window, &toolbar.borrow(), false);
            }
            end_text_input(&elements, &text_input_mode);
            draw.queue_draw();
            *current_tool.borrow_mut() = tool;
            cursor.show(tool, *color.borrow());
        },
    ));

    toolbar.borrow().connect_swatch_clicked(glib::clone!(
        #[strong]
        color,
        #[strong]
        toolbar,
        move || {
            let current = *color.borrow();
            toolbar.borrow().open_color_chooser(&current);
        },
    ));

    toolbar.borrow().connect_color_chosen(glib::clone!(
        #[strong]
        cursor,
        #[strong]
        color,
        #[strong]
        toolbar,
        #[strong]
        current_tool,
        #[strong]
        line_width,
        #[strong]
        elements,
        #[weak]
        draw,
        move |rgba| {
            *color.borrow_mut() = rgba;
            // Recolor the label being typed, like the scroll wheel resizes it.
            with_editing_label(&elements, |label| label.set_color(rgba));
            draw.queue_draw();
            let tool = *current_tool.borrow();
            cursor.show(tool, rgba);
            toolbar.borrow().update(&tool, &rgba, *line_width.borrow());
        },
    ));

    toolbar.borrow().connect_preset_selected(glib::clone!(
        #[strong]
        cursor,
        #[strong]
        color,
        #[strong]
        toolbar,
        #[strong]
        current_tool,
        #[strong]
        line_width,
        #[strong]
        elements,
        #[weak]
        draw,
        move |rgba| {
            *color.borrow_mut() = rgba;
            // Recolor the label being typed, like the scroll wheel resizes it.
            with_editing_label(&elements, |label| label.set_color(rgba));
            draw.queue_draw();
            let tool = *current_tool.borrow();
            cursor.show(tool, rgba);
            toolbar.borrow().update(&tool, &rgba, *line_width.borrow());
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
        #[strong]
        elements,
        #[weak]
        draw,
        move |delta| {
            let mut width = line_width.borrow_mut();
            let new_width = *width + delta;
            *width = if new_width < 1.0 { 1.0 } else { new_width };
            let w = *width;
            with_editing_label(&elements, |label| label.set_line_width(w));
            draw.queue_draw();
            let tool = *current_tool.borrow();
            let col = *color.borrow();
            toolbar.borrow().update(&tool, &col, *width);
        },
    ));

    toolbar.borrow().connect_undo(glib::clone!(
        #[strong]
        elements,
        #[strong]
        text_input_mode,
        #[weak]
        draw,
        move || {
            *text_input_mode.borrow_mut() = false;
            elements.borrow_mut().pop();
            draw.queue_draw();
        },
    ));

    toolbar.borrow().connect_pass_through(glib::clone!(
        #[weak]
        window,
        #[strong]
        toolbar,
        #[strong]
        elements,
        #[strong]
        text_input_mode,
        #[weak]
        draw,
        move |on| {
            end_text_input(&elements, &text_input_mode);
            draw.queue_draw();
            set_pass_through(&window, &toolbar.borrow(), on);
        },
    ));

    // Launching chicolli again (e.g. from the compositor shortcut) reaches this running
    // instance and brings it back from pass-through, with the drawing intact.
    application.connect_activate(glib::clone!(
        #[weak]
        window,
        #[strong]
        toolbar,
        move |_| set_pass_through(&window, &toolbar.borrow(), false),
    ));

    toolbar.borrow().connect_clear(glib::clone!(
        #[strong]
        elements,
        #[strong]
        text_input_mode,
        #[weak]
        draw,
        move || {
            *text_input_mode.borrow_mut() = false;
            elements.borrow_mut().clear();
            draw.queue_draw();
        },
    ));

    key_controller.connect_key_pressed(glib::clone!(
        #[strong]
        cursor,
        #[strong]
        draw,
        #[strong(rename_to = w)]
        window,
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
            // Let the color chooser popover handle its own typing (hex entry, Escape).
            if toolbar.borrow().color_chooser_open() {
                return Propagation::Proceed;
            }
            let is_shift = shift_after_key_event(keyval, modifier, true);
            *shift_held.borrow_mut() = is_shift;
            constrain_active(&elements, is_shift);
            draw.queue_draw();
            if *text_input_mode.borrow() {
                let ctrl = modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK);
                if ctrl && matches!(keyval, Key::v | Key::V) {
                    draw.clipboard().read_text_async(
                        None::<&gtk::gio::Cancellable>,
                        glib::clone!(
                            #[strong]
                            elements,
                            #[weak]
                            draw,
                            move |result| {
                                if let Ok(Some(text)) = result {
                                    with_editing_label(&elements, |label| label.push_str(&text));
                                    draw.queue_draw();
                                }
                            },
                        ),
                    );
                    return Propagation::Stop;
                }
                if ctrl {
                    // Other Ctrl shortcuts (undo, clear) end typing and then run as usual.
                    end_text_input(&elements, &text_input_mode);
                } else {
                    // Every other key is text, including the digits and letters that
                    // switch tools outside of typing. Escape or Return finish the label.
                    match keyval {
                        Key::Escape => end_text_input(&elements, &text_input_mode),
                        Key::Return | Key::KP_Enter if is_shift => {
                            with_editing_label(&elements, |label| label.push_char('\n'));
                        }
                        Key::Return | Key::KP_Enter => end_text_input(&elements, &text_input_mode),
                        Key::BackSpace => {
                            with_editing_label(&elements, |label| label.pop_char());
                        }
                        _ => {
                            if let Some(c) = keyval.to_unicode().filter(|c| !c.is_control()) {
                                with_editing_label(&elements, |label| label.push_char(c));
                            }
                        }
                    }
                    draw.queue_draw();
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
                }
                _ if _arrow_key == keyval => {
                    *current_tool.borrow_mut() =
                        drawing::drawing_tool::CurrentDrawingTool::NormalArrowHeadPointer;
                }
                _ if _reverse_arrow_key == keyval => {
                    *current_tool.borrow_mut() =
                        drawing::drawing_tool::CurrentDrawingTool::NormalArrowHeadBase;
                }
                _ if _rectangle_key == keyval => {
                    *current_tool.borrow_mut() =
                        drawing::drawing_tool::CurrentDrawingTool::NormalRectangle;
                }
                _ if _text_key == keyval => {
                    *current_tool.borrow_mut() =
                        drawing::drawing_tool::CurrentDrawingTool::TextLabel;
                }
                _ if _highlighter_key == keyval => {
                    *current_tool.borrow_mut() =
                        drawing::drawing_tool::CurrentDrawingTool::Highlighter;
                }
                _ if _disable_drawing_key == keyval => {
                    end_text_input(&elements, &text_input_mode);
                    draw.queue_draw();
                    set_pass_through(&w, &toolbar.borrow(), true);
                }
                // colors
                _ if _color_r == keyval => *color.borrow_mut() = colors::RED,
                _ if _color_g == keyval => *color.borrow_mut() = colors::GREEN,
                _ if _color_b == keyval => *color.borrow_mut() = colors::BLUE,
                _ if _undo_key == keyval
                    && modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK) =>
                {
                    *text_input_mode.borrow_mut() = false;
                    elements.borrow_mut().pop();
                    draw.queue_draw();
                }
                _ if _clear_all_key == keyval
                    && modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK) =>
                {
                    *text_input_mode.borrow_mut() = false;
                    elements.borrow_mut().clear();
                    draw.queue_draw();
                }
                _ if _color_chooser == keyval => {
                    let current = *color.borrow();
                    toolbar.borrow().open_color_chooser(&current);
                }
                _ => (),
            };
            let tool = *current_tool.borrow();
            let col = *color.borrow();
            let lw = *line_width.borrow();
            cursor.show(tool, col);
            toolbar.borrow().update(&tool, &col, lw);
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
        move |_, keyval, _, modifier| {
            let is_shift = shift_after_key_event(keyval, modifier, false);
            *shift_held.borrow_mut() = is_shift;
            constrain_active(&elements, is_shift);
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
        #[strong]
        shift_held,
        move |ctrl, x, y| {
            // The pointer event's modifier state is authoritative: it also catches Shift
            // pressed or released while keyboard focus was elsewhere.
            let shift = ctrl
                .current_event_state()
                .contains(gtk::gdk::ModifierType::SHIFT_MASK);
            *shift_held.borrow_mut() = shift;
            if let Some(elem) = elements.borrow_mut().last_mut() {
                if elem.active() {
                    elem.set_constrained(shift);
                }
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
        #[strong]
        shift_held,
        move |gesture, _, x, y| {
            // Clicking somewhere else ends typing into the previous label.
            end_text_input(&elements, &text_input_mode);
            let point = drawing::drawing_tool::Point(x, y);
            if *current_tool.borrow() == drawing::drawing_tool::CurrentDrawingTool::TextLabel {
                // Clicking an existing label picks it up: drag to move it, type to extend it.
                let mut elems = elements.borrow_mut();
                let hit = elems.iter_mut().rposition(|elem| {
                    elem.as_any_mut()
                        .downcast_mut::<drawing::text_label::TextLabel>()
                        .is_some_and(|label| label.contains(point))
                });
                if let Some(index) = hit {
                    // Move it to the end: the last element is the one being edited.
                    let mut elem = elems.remove(index);
                    if let Some(label) = elem
                        .as_any_mut()
                        .downcast_mut::<drawing::text_label::TextLabel>()
                    {
                        label.edit_and_grab(point);
                    }
                    elems.push(elem);
                    *text_input_mode.borrow_mut() = true;
                    draw.grab_focus();
                    draw.queue_draw();
                    return;
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
            drawing_tool.press_mouse(point);
            drawing_tool.set_line_width(*line_width.borrow());
            drawing_tool.set_color(*color.borrow());
            // Shift may already be held before the shape exists.
            let shift = gesture
                .current_event_state()
                .contains(gtk::gdk::ModifierType::SHIFT_MASK)
                || *shift_held.borrow();
            drawing_tool.set_constrained(shift);
            elements.borrow_mut().push(drawing_tool);
            draw.queue_draw();
        },
    ));

    left_click_mouse.connect_released(glib::clone!(
        #[strong]
        elements,
        #[weak]
        draw,
        move |_, _, x, y| {
            let mut elems = elements.borrow_mut();
            if let Some(elem) = elems.last_mut() {
                if !elem.active() {
                    return;
                }
                elem.release_mouse(drawing::drawing_tool::Point(x, y));
                // A click that drew nothing (e.g. an arrow without a drag) is dropped.
                if !elem.active() && elem.is_empty() {
                    elems.pop();
                }
            }
            draw.queue_draw();
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
            let tool = *current_tool.borrow();
            let col = *color.borrow();
            toolbar.borrow().update(&tool, &col, *width);
            Propagation::Proceed
        },
    ));

    draw.add_controller(scroll_controller);

    draw.set_draw_func(glib::clone!(
        #[weak]
        elements,
        move |_, ctx, _, _| {
            for element in elements.borrow().iter() {
                element.draw(ctx);
            }
        },
    ));

    // load css for the transparency of the window
    let provider = gtk::CssProvider::new();
    // `load_from_data` is deprecated from GTK 4.12, which the `hidpi-cursors` feature requires.
    #[cfg(feature = "hidpi-cursors")]
    provider.load_from_string(include_str!("styles/style.css"));
    #[cfg(not(feature = "hidpi-cursors"))]
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
        // A second launch only re-activates the existing overlay (see `activate`).
        if app.windows().is_empty() {
            activate(app);
        }
    });

    application.run();
}
