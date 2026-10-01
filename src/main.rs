use drawing::drawing_tool::{CurrentDrawingTool, DrawingTool};

use gtk::gio;
use gtk::glib::{self, Propagation};
use gtk::{
    cairo::{RectangleInt, Region},
    gdk::{Display, Key, Monitor},
    prelude::*,
};
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub mod colors;
pub mod config;
pub mod cursors;
pub mod drawing;
pub mod geometry;
pub mod keybinds;
pub mod toolbar;

/// A drawn element and the canvas (monitor) it was drawn on.
struct Element {
    canvas: u32,
    tool: Box<dyn DrawingTool>,
}

/// Everything drawn, on all monitors, oldest first, so Undo removes the latest
/// element wherever it is.
type Elements = Rc<RefCell<Vec<Element>>>;

/// The fullscreen overlay window on one monitor.
struct Canvas {
    id: u32,
    monitor: Monitor,
    window: gtk::ApplicationWindow,
    overlay: gtk::Overlay,
    draw: gtk::DrawingArea,
    cursor: cursors::ToolCursor,
}

/// State shared by the overlays on all monitors. There is one toolbar; it sits on the
/// monitor the pointer last entered.
#[derive(Clone)]
struct State {
    app: gtk::Application,
    conf: Rc<RefCell<config::Configuration>>,
    keybinds: Rc<Cell<keybinds::Keybinds>>,
    config_monitor: Rc<RefCell<Option<gio::FileMonitor>>>,
    elements: Elements,
    color: Rc<RefCell<colors::Color>>,
    current_tool: Rc<RefCell<CurrentDrawingTool>>,
    line_width: Rc<RefCell<f64>>,
    text_input_mode: Rc<RefCell<bool>>,
    shift_held: Rc<RefCell<bool>>,
    toolbar: Rc<toolbar::Toolbar>,
    canvases: Rc<RefCell<Vec<Canvas>>>,
    next_canvas_id: Rc<Cell<u32>>,
}

/// Whether Shift is down after this key event. The event's modifier state is the one
/// from before the key changed, so a Shift key's own press/release has to be applied.
fn shift_after_key_event(keyval: Key, modifier: gtk::gdk::ModifierType, pressed: bool) -> bool {
    match keyval {
        Key::Shift_L | Key::Shift_R => pressed,
        _ => modifier.contains(gtk::gdk::ModifierType::SHIFT_MASK),
    }
}

fn display() -> Display {
    Display::default().expect("error getting default display")
}

impl State {
    /// Ends typing into the text label being edited, if any, leaving it where it is.
    /// A label that ended up with no text is removed so Undo never pops something invisible.
    fn end_text_input(&self) {
        *self.text_input_mode.borrow_mut() = false;
        let mut elems = self.elements.borrow_mut();
        let Some(elem) = elems.last_mut() else {
            return;
        };
        let Some(label) = elem
            .tool
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
        &self,
        f: impl FnOnce(&mut drawing::text_label::TextLabel) -> R,
    ) -> Option<R> {
        let mut elems = self.elements.borrow_mut();
        let label = elems
            .last_mut()?
            .tool
            .as_any_mut()
            .downcast_mut::<drawing::text_label::TextLabel>()?;
        label.is_editing().then(|| f(label))
    }

    /// Applies the Shift constraint to the element currently being drawn.
    fn constrain_active(&self, shift: bool) {
        if let Some(elem) = self.elements.borrow_mut().last_mut() {
            if elem.tool.active() {
                elem.tool.set_constrained(shift);
            }
        }
    }

    fn redraw(&self) {
        for canvas in self.canvases.borrow().iter() {
            canvas.draw.queue_draw();
        }
    }

    /// Brings the cursors and the toolbar in line with the current tool, color and width.
    fn sync_ui(&self) {
        let tool = *self.current_tool.borrow();
        let col = *self.color.borrow();
        for canvas in self.canvases.borrow().iter() {
            canvas.cursor.show(tool, col);
        }
        self.toolbar.update(&tool, &col, *self.line_width.borrow());
    }

    fn undo(&self) {
        *self.text_input_mode.borrow_mut() = false;
        self.elements.borrow_mut().pop();
        self.redraw();
    }

    fn clear(&self) {
        *self.text_input_mode.borrow_mut() = false;
        self.elements.borrow_mut().clear();
        self.redraw();
    }

    /// Re-reads the config file after it changed on disk. Keybinds always follow the file;
    /// the line width only when `line_width` itself was edited, so a width picked on the
    /// toolbar survives unrelated edits. A file that fails to parse keeps the current settings.
    fn reload_config(&self, path: &std::path::Path) {
        let conf = match config::read_config_file(path) {
            Ok(conf) => conf,
            // Moved away or deleted; wait for the next version to appear.
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return,
            Err(err) => {
                eprintln!(
                    "chicolli: keeping the previous config, {}: {err}",
                    path.display()
                );
                return;
            }
        };
        self.keybinds.set(keybinds::Keybinds::from_config(&conf));
        if conf.line_width != self.conf.borrow().line_width {
            *self.line_width.borrow_mut() = conf.line_width;
            self.sync_ui();
        }
        *self.conf.borrow_mut() = conf;
    }

    /// Reloads the config whenever its file is saved, including editors that save by
    /// writing a temporary file and renaming it over the original.
    fn watch_config(&self) {
        let Some(path) = config::config_file_path() else {
            return;
        };
        let monitor = match gio::File::for_path(&path)
            .monitor_file(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
        {
            Ok(monitor) => monitor,
            Err(err) => {
                eprintln!(
                    "chicolli: not watching {} for changes: {err}",
                    path.display()
                );
                return;
            }
        };
        monitor.connect_changed(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move |_, _, _, event| {
                if matches!(
                    event,
                    gio::FileMonitorEvent::ChangesDoneHint
                        | gio::FileMonitorEvent::Renamed
                        | gio::FileMonitorEvent::MovedIn
                ) {
                    state.reload_config(&path);
                }
            },
        ));
        // The monitor stops when dropped; keep it for the life of the app.
        *self.config_monitor.borrow_mut() = Some(monitor);
    }

    fn set_color(&self, rgba: colors::Color) {
        *self.color.borrow_mut() = rgba;
        // Recolor the label being typed, like the scroll wheel resizes it.
        self.with_editing_label(|label| label.set_color(rgba));
        self.redraw();
        self.sync_ui();
    }

    /// Switches between drawing and pass-through. In pass-through the drawing stays on
    /// screen while clicks and keys go to the windows underneath; only the toolbar still
    /// takes clicks, so its pass-through toggle can switch back.
    fn set_pass_through(&self, on: bool) {
        self.toolbar.set_pass_through(on);
        for canvas in self.canvases.borrow().iter() {
            let window = &canvas.window;
            window.set_keyboard_mode(if on {
                KeyboardMode::None
            } else {
                KeyboardMode::Exclusive
            });
            if let Some(surface) = window.surface() {
                let region = if !on {
                    Region::create_rectangle(&RectangleInt::new(
                        0,
                        0,
                        surface.width(),
                        surface.height(),
                    ))
                } else {
                    // Only the overlay holding the toolbar has bounds for it; the
                    // others let every click through.
                    match self.toolbar.widget().compute_bounds(window) {
                        Some(b) => Region::create_rectangle(&RectangleInt::new(
                            b.x().floor() as i32,
                            b.y().floor() as i32,
                            b.width().ceil() as i32,
                            b.height().ceil() as i32,
                        )),
                        None => Region::create(),
                    }
                };
                surface.set_input_region(&region);
            }
            // Remap so the compositor picks up the new keyboard mode right away.
            window.unmap();
            window.map();
        }
    }

    /// Moves the toolbar onto the overlay of canvas `id`, unless it is already there.
    fn move_toolbar_to(&self, id: u32) {
        let widget = self.toolbar.widget();
        let canvases = self.canvases.borrow();
        let Some(target) = canvases.iter().find(|c| c.id == id) else {
            return;
        };
        let current = widget.parent().and_downcast::<gtk::Overlay>();
        if current.as_ref() == Some(&target.overlay) {
            return;
        }
        if let Some(parent) = current {
            parent.remove_overlay(widget);
        }
        target.overlay.add_overlay(widget);
    }

    /// Gives every monitor an overlay and closes the overlays of monitors that are gone,
    /// along with what was drawn on them.
    fn sync_monitors(&self) {
        let model = display().monitors();
        let monitors: Vec<Monitor> = (0..model.n_items())
            .filter_map(|i| model.item(i).and_downcast::<Monitor>())
            .collect();

        let gone: Vec<Canvas> = {
            let mut canvases = self.canvases.borrow_mut();
            let (keep, gone) = canvases
                .drain(..)
                .partition(|c| monitors.contains(&c.monitor));
            *canvases = keep;
            gone
        };
        if !gone.is_empty() {
            self.elements
                .borrow_mut()
                .retain(|e| !gone.iter().any(|c| c.id == e.canvas));
            // The label being typed may have been on a removed monitor.
            if self.with_editing_label(|_| ()).is_none() {
                *self.text_input_mode.borrow_mut() = false;
            }
            for canvas in gone {
                let widget = self.toolbar.widget();
                if widget.parent().as_ref() == Some(canvas.overlay.upcast_ref()) {
                    canvas.overlay.remove_overlay(widget);
                }
                canvas.window.destroy();
            }
        }

        for monitor in monitors {
            let known = self.canvases.borrow().iter().any(|c| c.monitor == monitor);
            if !known {
                self.add_canvas(&monitor);
            }
        }

        if self.toolbar.widget().parent().is_none() {
            let first = self.canvases.borrow().first().map(|c| c.id);
            if let Some(id) = first {
                self.move_toolbar_to(id);
            }
        }
    }

    /// Opens a fullscreen overlay on `monitor`.
    fn add_canvas(&self, monitor: &Monitor) {
        let id = self.next_canvas_id.get();
        self.next_canvas_id.set(id + 1);

        // Create a normal GTK window however you like
        let window = gtk::ApplicationWindow::new(&self.app);

        // Before the window is first realized, set it up to be a layer surface
        window.init_layer_shell();
        window.set_monitor(Some(monitor));
        let pass_through = self.toolbar.pass_through();
        window.set_keyboard_mode(if pass_through {
            KeyboardMode::None
        } else {
            KeyboardMode::Exclusive
        });
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

        // Set up a widget
        let draw = gtk::DrawingArea::new();
        draw.set_focusable(true);
        let cursor = cursors::ToolCursor::new(&draw);
        cursor.show(*self.current_tool.borrow(), *self.color.borrow());

        let key_controller = gtk::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        key_controller.connect_key_pressed(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move |_, keyval, _, modifier| state.key_pressed(keyval, modifier),
        ));
        key_controller.connect_key_released(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move |_, keyval, _, modifier| {
                let is_shift = shift_after_key_event(keyval, modifier, false);
                *state.shift_held.borrow_mut() = is_shift;
                state.constrain_active(is_shift);
                state.redraw();
            },
        ));

        // key controller is added to the window and not to the drawarea because there it
        // does not work
        window.add_controller(key_controller);

        let motion_controller = gtk::EventControllerMotion::new();
        motion_controller.connect_enter(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move |_, _, _| {
                // The toolbar follows the pointer to the monitor it is drawing on. It
                // stays put while its color chooser is open.
                if !state.toolbar.color_chooser_open() {
                    state.move_toolbar_to(id);
                }
            },
        ));
        motion_controller.connect_motion(glib::clone!(
            #[weak]
            draw,
            #[strong(rename_to = state)]
            self,
            move |ctrl, x, y| {
                // The pointer event's modifier state is authoritative: it also catches Shift
                // pressed or released while keyboard focus was elsewhere.
                let shift = ctrl
                    .current_event_state()
                    .contains(gtk::gdk::ModifierType::SHIFT_MASK);
                *state.shift_held.borrow_mut() = shift;
                if let Some(elem) = state.elements.borrow_mut().last_mut() {
                    if elem.canvas != id {
                        return;
                    }
                    if elem.tool.active() {
                        elem.tool.set_constrained(shift);
                    }
                    elem.tool.motion_notify(drawing::drawing_tool::Point(x, y));
                    if elem.tool.active() {
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

        // Set the gestures button to the left mouse button (=1)
        left_click_mouse.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);

        left_click_mouse.connect_pressed(glib::clone!(
            #[strong(rename_to = state)]
            self,
            #[weak]
            draw,
            move |gesture, _, x, y| state.press(id, &draw, gesture, x, y),
        ));

        left_click_mouse.connect_released(glib::clone!(
            #[strong(rename_to = state)]
            self,
            #[weak]
            draw,
            move |_, _, x, y| {
                let mut elems = state.elements.borrow_mut();
                if let Some(elem) = elems.last_mut() {
                    if elem.canvas != id || !elem.tool.active() {
                        return;
                    }
                    elem.tool.release_mouse(drawing::drawing_tool::Point(x, y));
                    // A click that drew nothing (e.g. an arrow without a drag) is dropped.
                    if !elem.tool.active() && elem.tool.is_empty() {
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
            #[strong(rename_to = state)]
            self,
            move |_, _, scroll| {
                let width = {
                    let mut width = state.line_width.borrow_mut();
                    let new_width = *width - scroll;
                    *width = if new_width as i32 >= 1 {
                        new_width
                    } else {
                        1.0
                    };
                    *width
                };
                if *state.text_input_mode.borrow() {
                    state.with_editing_label(|label| label.set_line_width(width));
                    state.redraw();
                }
                state.sync_ui();
                Propagation::Proceed
            },
        ));

        draw.add_controller(scroll_controller);

        draw.set_draw_func(glib::clone!(
            #[weak(rename_to = elements)]
            self.elements,
            move |_, ctx, _, _| {
                for element in elements.borrow().iter().filter(|e| e.canvas == id) {
                    element.tool.draw(ctx);
                }
            },
        ));

        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&draw));

        window.set_child(Some(&overlay));
        window.set_visible(true);

        if pass_through {
            // A monitor plugged in during pass-through must not catch clicks.
            if let Some(surface) = window.surface() {
                surface.set_input_region(&Region::create());
            }
        }

        self.canvases.borrow_mut().push(Canvas {
            id,
            monitor: monitor.clone(),
            window,
            overlay,
            draw,
            cursor,
        });
    }

    /// Starts a new element, or picks up a text label, where canvas `id` was clicked.
    fn press(&self, id: u32, draw: &gtk::DrawingArea, gesture: &gtk::GestureClick, x: f64, y: f64) {
        // Clicking somewhere else ends typing into the previous label.
        self.end_text_input();
        let point = drawing::drawing_tool::Point(x, y);
        let current_tool = *self.current_tool.borrow();
        if current_tool == CurrentDrawingTool::TextLabel {
            // Clicking an existing label picks it up: drag to move it, type to extend it.
            let mut elems = self.elements.borrow_mut();
            let hit = elems.iter_mut().rposition(|elem| {
                elem.canvas == id
                    && elem
                        .tool
                        .as_any_mut()
                        .downcast_mut::<drawing::text_label::TextLabel>()
                        .is_some_and(|label| label.contains(point))
            });
            if let Some(index) = hit {
                // Move it to the end: the last element is the one being edited.
                let mut elem = elems.remove(index);
                if let Some(label) = elem
                    .tool
                    .as_any_mut()
                    .downcast_mut::<drawing::text_label::TextLabel>()
                {
                    label.edit_and_grab(point);
                }
                elems.push(elem);
                *self.text_input_mode.borrow_mut() = true;
                draw.grab_focus();
                draw.queue_draw();
                return;
            }
        }
        let mut drawing_tool: Box<dyn DrawingTool> = match current_tool {
            CurrentDrawingTool::NormalLine => Box::new(drawing::normal_line::NormalLine::new()),
            CurrentDrawingTool::NormalArrowHeadBase => {
                Box::new(drawing::arrow::NormalArrow::new(true))
            }
            CurrentDrawingTool::NormalArrowHeadPointer => {
                Box::new(drawing::arrow::NormalArrow::new(false))
            }
            CurrentDrawingTool::NormalRectangle => {
                Box::new(drawing::normal_rectangle::NormalRectangle::new())
            }
            CurrentDrawingTool::Highlighter => Box::new(drawing::highlighter::Highlighter::new()),
            CurrentDrawingTool::TextLabel => {
                *self.text_input_mode.borrow_mut() = true;
                draw.grab_focus();
                Box::new(drawing::text_label::TextLabel::new())
            }
        };
        drawing_tool.press_mouse(point);
        drawing_tool.set_line_width(*self.line_width.borrow());
        drawing_tool.set_color(*self.color.borrow());
        // Shift may already be held before the shape exists.
        let shift = gesture
            .current_event_state()
            .contains(gtk::gdk::ModifierType::SHIFT_MASK)
            || *self.shift_held.borrow();
        drawing_tool.set_constrained(shift);
        self.elements.borrow_mut().push(Element {
            canvas: id,
            tool: drawing_tool,
        });
        draw.queue_draw();
    }

    fn key_pressed(&self, keyval: Key, modifier: gtk::gdk::ModifierType) -> Propagation {
        // Let the color chooser popover handle its own typing (hex entry, Escape).
        if self.toolbar.color_chooser_open() {
            return Propagation::Proceed;
        }
        let is_shift = shift_after_key_event(keyval, modifier, true);
        *self.shift_held.borrow_mut() = is_shift;
        self.constrain_active(is_shift);
        self.redraw();
        if *self.text_input_mode.borrow() {
            let ctrl = modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK);
            if ctrl && matches!(keyval, Key::v | Key::V) {
                display().clipboard().read_text_async(
                    None::<&gtk::gio::Cancellable>,
                    glib::clone!(
                        #[strong(rename_to = state)]
                        self,
                        move |result| {
                            if let Ok(Some(text)) = result {
                                state.with_editing_label(|label| label.push_str(&text));
                                state.redraw();
                            }
                        },
                    ),
                );
                return Propagation::Stop;
            }
            if ctrl {
                // Other Ctrl shortcuts (undo, clear) end typing and then run as usual.
                self.end_text_input();
            } else {
                // Every other key is text, including the digits and letters that
                // switch tools outside of typing. Escape or Return finish the label.
                match keyval {
                    Key::Escape => self.end_text_input(),
                    Key::Return | Key::KP_Enter if is_shift => {
                        self.with_editing_label(|label| label.push_char('\n'));
                    }
                    Key::Return | Key::KP_Enter => self.end_text_input(),
                    Key::BackSpace => {
                        self.with_editing_label(|label| label.pop_char());
                    }
                    _ => {
                        if let Some(c) = keyval.to_unicode().filter(|c| !c.is_control()) {
                            self.with_editing_label(|label| label.push_char(c));
                        }
                    }
                }
                self.redraw();
                return Propagation::Stop;
            }
        }

        let keys = self.keybinds.get();
        let hit = |binding: Option<keybinds::Binding>| {
            binding.is_some_and(|binding| binding.matches(keyval, modifier))
        };
        match keyval {
            // TOOLS
            _ if hit(keys.pen) => {
                *self.current_tool.borrow_mut() = CurrentDrawingTool::NormalLine;
            }
            _ if hit(keys.arrow) => {
                *self.current_tool.borrow_mut() = CurrentDrawingTool::NormalArrowHeadPointer;
            }
            _ if hit(keys.reverse_arrow) => {
                *self.current_tool.borrow_mut() = CurrentDrawingTool::NormalArrowHeadBase;
            }
            _ if hit(keys.rectangle) => {
                *self.current_tool.borrow_mut() = CurrentDrawingTool::NormalRectangle;
            }
            _ if hit(keys.text) => {
                *self.current_tool.borrow_mut() = CurrentDrawingTool::TextLabel;
            }
            _ if hit(keys.highlighter) => {
                *self.current_tool.borrow_mut() = CurrentDrawingTool::Highlighter;
            }
            _ if hit(keys.pass_through) => {
                self.end_text_input();
                self.redraw();
                self.set_pass_through(true);
            }
            // colors
            _ if hit(keys.red) => *self.color.borrow_mut() = colors::RED,
            _ if hit(keys.green) => *self.color.borrow_mut() = colors::GREEN,
            _ if hit(keys.blue) => *self.color.borrow_mut() = colors::BLUE,
            _ if hit(keys.undo) => self.undo(),
            _ if hit(keys.clear) => self.clear(),
            _ if hit(keys.color_chooser) => {
                let current = *self.color.borrow();
                self.toolbar.open_color_chooser(&current);
            }
            _ => (),
        };
        self.sync_ui();
        Propagation::Proceed
    }

    fn connect_toolbar(&self) {
        let toolbar = &self.toolbar;

        toolbar.connect_tool_selected(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move |tool| {
                // Picking a tool while passing clicks through means the user wants to draw again.
                if state.toolbar.pass_through() {
                    state.set_pass_through(false);
                }
                state.end_text_input();
                state.redraw();
                *state.current_tool.borrow_mut() = tool;
                let col = *state.color.borrow();
                for canvas in state.canvases.borrow().iter() {
                    canvas.cursor.show(tool, col);
                }
            },
        ));

        toolbar.connect_swatch_clicked(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move || {
                let current = *state.color.borrow();
                state.toolbar.open_color_chooser(&current);
            },
        ));

        toolbar.connect_color_chosen(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move |rgba| state.set_color(rgba),
        ));

        toolbar.connect_preset_selected(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move |rgba| state.set_color(rgba),
        ));

        toolbar.connect_line_width_changed(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move |delta| {
                let width = {
                    let mut width = state.line_width.borrow_mut();
                    *width = (*width + delta).max(1.0);
                    *width
                };
                state.with_editing_label(|label| label.set_line_width(width));
                state.redraw();
                state.sync_ui();
            },
        ));

        toolbar.connect_undo(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move || state.undo(),
        ));

        toolbar.connect_clear(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move || state.clear(),
        ));

        toolbar.connect_pass_through(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move |on| {
                state.end_text_input();
                state.redraw();
                state.set_pass_through(on);
            },
        ));
    }
}

// https://github.com/wmww/gtk-layer-shell/blob/master/examples/simple-example.c
fn activate(application: &gtk::Application) {
    let conf = config::get_config();
    let line_width = conf.line_width;

    let toolbar = toolbar::Toolbar::new();
    toolbar.update(&CurrentDrawingTool::NormalLine, &colors::RED, line_width);

    let state = State {
        app: application.clone(),
        keybinds: Rc::new(Cell::new(keybinds::Keybinds::from_config(&conf))),
        conf: Rc::new(RefCell::new(conf)),
        config_monitor: Rc::new(RefCell::new(None)),
        elements: Rc::new(RefCell::new(Vec::new())),
        color: Rc::new(RefCell::new(colors::RED)),
        current_tool: Rc::new(RefCell::new(CurrentDrawingTool::NormalLine)),
        line_width: Rc::new(RefCell::new(line_width)),
        text_input_mode: Rc::new(RefCell::new(false)),
        shift_held: Rc::new(RefCell::new(false)),
        toolbar: Rc::new(toolbar),
        canvases: Rc::new(RefCell::new(Vec::new())),
        next_canvas_id: Rc::new(Cell::new(0)),
    };
    state.connect_toolbar();
    state.watch_config();

    // Launching chicolli again (e.g. from the compositor shortcut) reaches this running
    // instance and brings it back from pass-through, with the drawing intact.
    application.connect_activate(glib::clone!(
        #[strong]
        state,
        move |_| state.set_pass_through(false),
    ));

    // load css for the transparency of the window
    let provider = gtk::CssProvider::new();
    // `load_from_data` is deprecated from GTK 4.12, which the `hidpi-cursors` feature requires.
    #[cfg(feature = "hidpi-cursors")]
    provider.load_from_string(include_str!("styles/style.css"));
    #[cfg(not(feature = "hidpi-cursors"))]
    provider.load_from_data(include_str!("styles/style.css"));
    gtk::style_context_add_provider_for_display(
        &display(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // One overlay per monitor, kept in step as monitors are plugged in and out.
    state.sync_monitors();
    display().monitors().connect_items_changed(glib::clone!(
        #[strong]
        state,
        move |_, _, _, _| state.sync_monitors(),
    ));
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
