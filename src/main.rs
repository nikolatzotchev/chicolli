use drawing::drawing_tool::{CurrentDrawingTool, DrawingTool, Point};
use geometry::distance_sq;

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

pub mod capture;
pub mod colors;
pub mod config;
pub mod cursors;
pub mod drawing;
pub mod geometry;
pub mod history;
pub mod keybinds;
pub mod toolbar;

/// A drawn element and the canvas (monitor) it was started on. Elements are kept in
/// global layout coordinates, so a stroke dragged across a monitor edge carries on onto
/// the next monitor, whatever their resolutions and scales.
struct Element {
    canvas: u32,
    tool: Box<dyn DrawingTool>,
}

impl history::Movable for Element {
    fn move_by(&mut self, by: Point) {
        self.tool.translate(by);
    }
}

/// Index of the element the select tool picks up at `point`: the topmost one drawn
/// there, unless an eraser stroke above it wiped that spot clean.
fn pick<'a>(
    mut elements: impl DoubleEndedIterator<Item = &'a Element> + ExactSizeIterator,
    point: Point,
) -> Option<usize> {
    let mut index = elements.len();
    while let Some(element) = elements.next_back() {
        index -= 1;
        if element.tool.erases(point) {
            return None;
        }
        if element.tool.hit(point) {
            return Some(index);
        }
    }
    None
}

/// An element picked up with the select tool, while the button is held.
#[derive(Clone, Copy)]
struct Drag {
    index: usize,
    /// Where the button went down.
    start: Point,
    /// Where the element has been moved to follow, once the pointer left the click
    /// threshold; until then the press may still be a click, which deletes the element.
    last: Option<Point>,
}

/// Everything drawn, on all monitors, oldest first, so Undo removes the latest
/// element wherever it is, with what Undo and Clear took away so Undo and Redo can
/// bring it back.
type Elements = Rc<RefCell<history::History<Element>>>;

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
    /// A screenshot is being taken; further copy/save presses wait for it.
    capturing: Rc<Cell<bool>>,
    /// slurp is picking a region, so the overlays take no input.
    selecting_region: Rc<Cell<bool>>,
    /// Where the pointer is over the overlays, in global layout coordinates, for the
    /// eraser's outline.
    pointer: Rc<Cell<Option<Point>>>,
    drag: Rc<Cell<Option<Drag>>>,
}

/// The id copy/save notifications share, so each replaces the one before.
const CAPTURE_NOTIFICATION: &str = "chicolli-capture";

/// What to do with a screenshot of the annotated desktop.
#[derive(Clone, Copy)]
enum Capture {
    Copy,
    Save,
}

/// Which input the overlays take.
#[derive(Clone, Copy)]
enum Input {
    /// Everything: drawing mode.
    Drawing,
    /// Only clicks on the toolbar.
    PassThrough,
    /// Nothing, while slurp picks a region.
    None,
}

/// Whether Shift is down after this key event. The event's modifier state is the one
/// from before the key changed, so a Shift key's own press/release has to be applied.
fn shift_after_key_event(keyval: Key, modifier: gtk::gdk::ModifierType, pressed: bool) -> bool {
    match keyval {
        Key::Shift_L | Key::Shift_R => pressed,
        _ => modifier.contains(gtk::gdk::ModifierType::SHIFT_MASK),
    }
}

/// Where `monitor`'s overlay sits in the global (logical pixel) layout.
fn origin(monitor: &Monitor) -> Point {
    let geometry = monitor.geometry();
    Point(f64::from(geometry.x()), f64::from(geometry.y()))
}

fn display() -> Display {
    Display::default().expect("error getting default display")
}

impl State {
    /// Ends typing into the text label being edited, if any, leaving it where it is.
    /// A label that ended up with no text is removed so Undo never pops something invisible.
    fn end_text_input(&self) {
        *self.text_input_mode.borrow_mut() = false;
        let mut history = self.elements.borrow_mut();
        let Some(elem) = history.last_mut() else {
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
            history.pop();
        } else {
            label.commit();
            history.commit();
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

    /// Whether the overlays draw something at the pointer (the eraser's size, the
    /// select tool's outline), so they need redrawing as it moves.
    fn shows_pointer(&self) -> bool {
        matches!(
            *self.current_tool.borrow(),
            CurrentDrawingTool::Eraser | CurrentDrawingTool::Select
        )
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
        // The eraser outline follows the tool and the width.
        self.redraw();
    }

    /// Drops the element picked up with the select tool where it is now. A move so far
    /// is kept, as one undo step; a press that has not moved yet is not a click and
    /// deletes nothing.
    fn end_drag(&self) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        if let Some(last) = drag
            .last
            .filter(|last| distance_sq(*last, drag.start) > 0.0)
        {
            self.elements
                .borrow_mut()
                .moved(drag.index, last - drag.start);
        }
    }

    /// Picks up the topmost element under `point` with the select tool.
    fn select_press(&self, point: Point) {
        let hit = pick(self.elements.borrow().items(), point);
        if let Some(index) = hit {
            self.drag.set(Some(Drag {
                index,
                start: point,
                last: None,
            }));
        }
    }

    /// Moves the element picked up with the select tool along with the pointer, once
    /// the pointer has left the click threshold. Returns whether something is picked up.
    fn select_motion(&self, point: Point) -> bool {
        let Some(mut drag) = self.drag.get() else {
            return false;
        };
        let from = match drag.last {
            Some(last) => last,
            None if distance_sq(point, drag.start)
                < drawing::selection::DRAG_THRESHOLD * drawing::selection::DRAG_THRESHOLD =>
            {
                return true;
            }
            None => drag.start,
        };
        if let Some(elem) = self.elements.borrow_mut().get_mut(drag.index) {
            elem.tool.translate(point - from);
        }
        drag.last = Some(point);
        self.drag.set(Some(drag));
        true
    }

    /// Lets go of the element picked up with the select tool: a drag leaves it where it
    /// was moved to, a click deletes it. Both can be undone.
    fn select_release(&self, point: Point) {
        self.select_motion(point);
        let Some(drag) = self.drag.get() else {
            return;
        };
        if drag.last.is_some() {
            self.end_drag();
        } else {
            self.drag.set(None);
            self.elements.borrow_mut().delete(drag.index);
        }
    }

    /// Takes back the latest element or Clear. Undo while typing into a new, still empty
    /// label just drops that label.
    fn undo(&self) {
        let typing_nothing = self.with_editing_label(|label| label.is_empty()) == Some(true);
        self.end_text_input();
        self.end_drag();
        if !typing_nothing {
            self.elements.borrow_mut().undo();
        }
        self.redraw();
    }

    fn redo(&self) {
        self.end_text_input();
        self.end_drag();
        // Not in the middle of a drag: the element being drawn has to stay last.
        let mut history = self.elements.borrow_mut();
        if history.last_mut().is_some_and(|e| e.tool.active()) {
            return;
        }
        history.redo();
        drop(history);
        self.redraw();
    }

    /// Removes everything; Undo brings it back.
    fn clear(&self) {
        self.end_text_input();
        self.end_drag();
        self.elements.borrow_mut().clear();
        self.redraw();
    }

    /// Screenshots the desktop with the drawing on it, without the toolbar, and copies
    /// it to the clipboard or saves it in the pictures folder. With `region`, the user
    /// first drags out the part to take with `slurp`.
    fn capture(&self, what: Capture, region: bool) {
        // A stroke still being drawn would never see its button come up once slurp
        // has the pointer; take the region after letting go.
        let drawing = !*self.text_input_mode.borrow()
            && self
                .elements
                .borrow_mut()
                .last_mut()
                .is_some_and(|e| e.tool.active());
        if (region && drawing) || self.capturing.replace(true) {
            return;
        }
        // Finish the label being typed so its caret is not in the picture.
        self.end_text_input();
        self.end_drag();
        self.redraw();
        let toolbar = self.toolbar.widget().clone();
        toolbar.set_visible(false);
        if region {
            // slurp's own overlay has to get the pointer and keyboard (Escape cancels).
            self.selecting_region.set(true);
            self.set_input(Input::None);
        }
        // The last copy/save notification would be in the picture too.
        self.app.withdraw_notification(CAPTURE_NOTIFICATION);

        let finish = glib::clone!(
            #[strong(rename_to = state)]
            self,
            move |png: Option<capture::Screenshot>| {
                toolbar.set_visible(true);
                if region {
                    state.selecting_region.set(false);
                    state.set_input(if state.toolbar.pass_through() {
                        Input::PassThrough
                    } else {
                        Input::Drawing
                    });
                    // slurp got the release of Ctrl+Shift.
                    *state.shift_held.borrow_mut() = false;
                }
                state.capturing.set(false);
                let subject = if region {
                    "the selection"
                } else {
                    "the screen"
                };
                match (png, what) {
                    // The selection was cancelled.
                    (None, _) => (),
                    (Some(Err(err)), _) => state.notify(Err(err)),
                    (Some(Ok(png)), Capture::Copy) => {
                        let state = state.clone();
                        capture::copy_to_clipboard(png, display(), subject, move |result| {
                            state.notify(result)
                        });
                    }
                    (Some(Ok(png)), Capture::Save) => state.notify(
                        capture::save(&png)
                            .map(|path| format!("Saved {subject} to {}", path.display())),
                    ),
                }
            }
        );
        // Give the compositor a moment to show the overlays without the toolbar and
        // the notification.
        glib::timeout_add_local_once(std::time::Duration::from_millis(200), move || {
            if !region {
                return capture::screenshot(move |png| finish(Some(png)));
            }
            capture::select_region(move |selection| match selection {
                Ok(Some(geometry)) => {
                    // And a moment for slurp's dimmed overlay to go away.
                    glib::timeout_add_local_once(
                        std::time::Duration::from_millis(100),
                        move || capture::screenshot_region(&geometry, move |png| finish(Some(png))),
                    );
                }
                Ok(None) => {
                    eprintln!("chicolli: region selection cancelled");
                    finish(None)
                }
                Err(err) => finish(Some(Err(err))),
            });
        });
    }

    /// Reports how a copy or save went, on stderr and as a desktop notification.
    fn notify(&self, result: Result<String, String>) {
        let notification = match &result {
            Ok(message) => {
                eprintln!("chicolli: {message}");
                gio::Notification::new(message)
            }
            Err(err) => {
                eprintln!("chicolli: could not capture the screen: {err}");
                let notification = gio::Notification::new("Could not capture the screen");
                notification.set_body(Some(err));
                notification
            }
        };
        self.app
            .send_notification(Some(CAPTURE_NOTIFICATION), &notification);
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
        let keys = keybinds::Keybinds::from_config(&conf);
        self.keybinds.set(keys);
        self.toolbar.show_keybinds(&keys);
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
        // The overlays may not see the button come up any more.
        self.end_drag();
        self.toolbar.set_pass_through(on);
        // slurp keeps the input; the capture applies the toolbar's mode when it ends.
        if self.selecting_region.get() {
            return;
        }
        self.set_input(if on {
            Input::PassThrough
        } else {
            Input::Drawing
        });
    }

    /// Sets which input the overlays take: everything, only toolbar clicks, or nothing.
    fn set_input(&self, input: Input) {
        for canvas in self.canvases.borrow().iter() {
            let window = &canvas.window;
            window.set_keyboard_mode(match input {
                Input::Drawing => KeyboardMode::Exclusive,
                Input::PassThrough | Input::None => KeyboardMode::None,
            });
            if let Some(surface) = window.surface() {
                let region = match input {
                    Input::Drawing => Region::create_rectangle(&RectangleInt::new(
                        0,
                        0,
                        surface.width(),
                        surface.height(),
                    )),
                    // Only the overlay holding the toolbar has bounds for it; the
                    // others let every click through.
                    Input::PassThrough => match self.toolbar.widget().compute_bounds(window) {
                        Some(b) => Region::create_rectangle(&RectangleInt::new(
                            b.x().floor() as i32,
                            b.y().floor() as i32,
                            b.width().ceil() as i32,
                            b.height().ceil() as i32,
                        )),
                        None => Region::create(),
                    },
                    Input::None => Region::create(),
                };
                surface.set_input_region(Some(&region));
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
            // Its index may not hold once elements go.
            self.end_drag();
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
        let monitor = monitor.clone();
        let id = self.next_canvas_id.get();
        self.next_canvas_id.set(id + 1);

        // Create a normal GTK window however you like
        let window = gtk::ApplicationWindow::new(&self.app);

        // Before the window is first realized, set it up to be a layer surface
        window.init_layer_shell();
        window.set_monitor(Some(&monitor));
        // A monitor plugged in during pass-through or while slurp picks a region must
        // not catch clicks or keys.
        let pass_through = self.toolbar.pass_through() || self.selecting_region.get();
        window.set_keyboard_mode(if pass_through {
            KeyboardMode::None
        } else {
            KeyboardMode::Exclusive
        });
        // Display above normal windows
        window.set_layer(Layer::Overlay);
        // Lets compositor rules (e.g. Hyprland's `layerrule`) match the overlays.
        window.set_namespace(Some("chicolli"));
        // Cover panels' exclusive zones too, so the overlay spans the whole monitor
        // and its origin is the monitor's geometry origin the element points assume.
        window.set_exclusive_zone(-1);
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
        motion_controller.connect_leave(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move |_| {
                state.pointer.set(None);
                if state.shows_pointer() {
                    state.redraw();
                }
            },
        ));
        motion_controller.connect_motion(glib::clone!(
            #[strong]
            monitor,
            #[strong(rename_to = state)]
            self,
            move |ctrl, x, y| {
                let point = origin(&monitor) + Point(x, y);
                state.pointer.set(Some(point));
                // The pointer event's modifier state is authoritative: it also catches Shift
                // pressed or released while keyboard focus was elsewhere.
                let shift = ctrl
                    .current_event_state()
                    .contains(gtk::gdk::ModifierType::SHIFT_MASK);
                *state.shift_held.borrow_mut() = shift;
                // A grab that took the release away (a popover, the compositor) leaves
                // the drag without its button.
                if !ctrl
                    .current_event_state()
                    .contains(gtk::gdk::ModifierType::BUTTON1_MASK)
                {
                    state.end_drag();
                }
                if state.select_motion(point) {
                    state.redraw();
                    return;
                }
                // While a button is held the pressed overlay keeps getting the motion,
                // also past its monitor's edge, so the element follows the pointer
                // onto the other monitors.
                let active = state.elements.borrow_mut().last_mut().is_some_and(|elem| {
                    if elem.tool.active() {
                        elem.tool.set_constrained(shift);
                    }
                    elem.tool.motion_notify(point);
                    elem.tool.active()
                });
                if active || state.shows_pointer() {
                    state.redraw();
                }
            },
        ));

        draw.add_controller(motion_controller);

        let right_click_mouse = gtk::GestureClick::new();

        // Set the gestures button to the right mouse button (=3)
        right_click_mouse.set_button(gtk::gdk::ffi::GDK_BUTTON_SECONDARY as u32);

        // Assign your handler to an event of the gesture (e.g. the `pressed` event)
        right_click_mouse.connect_pressed(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move |_, _, _, _| state.app.quit(),
        ));

        draw.add_controller(right_click_mouse);

        let left_click_mouse = gtk::GestureClick::new();

        // Set the gestures button to the left mouse button (=1)
        left_click_mouse.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);

        left_click_mouse.connect_pressed(glib::clone!(
            #[strong(rename_to = state)]
            self,
            #[weak]
            draw,
            #[strong]
            monitor,
            move |gesture, _, x, y| {
                state.press(id, &draw, gesture, origin(&monitor) + Point(x, y))
            },
        ));

        left_click_mouse.connect_released(glib::clone!(
            #[strong(rename_to = state)]
            self,
            #[strong]
            monitor,
            move |_, _, x, y| {
                let point = origin(&monitor) + Point(x, y);
                if state.drag.get().is_some() {
                    state.select_release(point);
                    state.redraw();
                    return;
                }
                {
                    let mut history = state.elements.borrow_mut();
                    let Some(elem) = history.last_mut() else {
                        return;
                    };
                    if !elem.tool.active() {
                        return;
                    }
                    elem.tool.release_mouse(point);
                    // A click that drew nothing (e.g. an arrow without a drag) is dropped
                    // and leaves Redo as it was. Labels stay active, and count, until
                    // typing ends.
                    if !elem.tool.active() {
                        if elem.tool.is_empty() {
                            history.pop();
                        } else {
                            history.commit();
                        }
                    }
                }
                state.redraw();
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
                    *width =
                        (*width - scroll).clamp(config::MIN_LINE_WIDTH, config::MAX_LINE_WIDTH);
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
            #[strong(rename_to = tool)]
            self.current_tool,
            #[strong(rename_to = pointer)]
            self.pointer,
            #[strong(rename_to = width)]
            self.line_width,
            #[strong(rename_to = capturing)]
            self.capturing,
            #[strong(rename_to = drag)]
            self.drag,
            #[strong]
            monitor,
            move |_, ctx, _, _| {
                let Point(x, y) = origin(&monitor);
                ctx.translate(-x, -y);
                let elements = elements.borrow();
                for element in elements.items() {
                    element.tool.draw(ctx);
                }
                // Tool feedback stays out of screenshots.
                if capturing.get() {
                    return;
                }
                match *tool.borrow() {
                    // Show how much the eraser takes.
                    CurrentDrawingTool::Eraser => {
                        if let Some(point) = pointer.get() {
                            drawing::eraser::draw_outline(ctx, point, *width.borrow());
                        }
                    }
                    // Outline what a click or drag would pick up, or has picked up.
                    CurrentDrawingTool::Select => {
                        let picked = match drag.get() {
                            Some(drag) => Some(drag.index),
                            None => pointer.get().and_then(|p| pick(elements.items(), p)),
                        };
                        if let Some(bounds) = picked
                            .and_then(|i| elements.items().nth(i))
                            .and_then(|e| e.tool.bounds())
                        {
                            drawing::selection::draw_outline(ctx, bounds);
                        }
                    }
                    _ => {}
                }
            },
        ));

        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&draw));

        window.set_child(Some(&overlay));
        window.set_visible(true);

        if pass_through {
            if let Some(surface) = window.surface() {
                surface.set_input_region(Some(&Region::create()));
            }
        }

        self.canvases.borrow_mut().push(Canvas {
            id,
            monitor,
            window,
            overlay,
            draw,
            cursor,
        });
    }

    /// Starts a new element, or picks up a text label, where canvas `id` was clicked at
    /// `point` (in global layout coordinates).
    fn press(&self, id: u32, draw: &gtk::DrawingArea, gesture: &gtk::GestureClick, point: Point) {
        // Clicking somewhere else ends typing into the previous label.
        self.end_text_input();
        // A drag whose release never arrived ends where it is.
        self.end_drag();
        let current_tool = *self.current_tool.borrow();
        if current_tool == CurrentDrawingTool::Select {
            self.select_press(point);
            self.redraw();
            return;
        }
        if current_tool == CurrentDrawingTool::TextLabel {
            // Clicking an existing label picks it up: drag to move it, type to extend it.
            let mut history = self.elements.borrow_mut();
            let hit = history.rposition_mut(|elem| {
                elem.tool
                    .as_any_mut()
                    .downcast_mut::<drawing::text_label::TextLabel>()
                    .is_some_and(|label| label.contains(point))
            });
            if let Some(index) = hit {
                // Move it to the end: the last element is the one being edited.
                history.raise(index);
                if let Some(label) = history.last_mut().and_then(|elem| {
                    elem.tool
                        .as_any_mut()
                        .downcast_mut::<drawing::text_label::TextLabel>()
                }) {
                    label.edit_and_grab(point);
                }
                *self.text_input_mode.borrow_mut() = true;
                draw.grab_focus();
                self.redraw();
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
            CurrentDrawingTool::Eraser => Box::new(drawing::eraser::Eraser::new()),
            CurrentDrawingTool::Select => return,
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
        self.redraw();
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
            _ if hit(keys.eraser) => {
                *self.current_tool.borrow_mut() = CurrentDrawingTool::Eraser;
            }
            _ if hit(keys.select) => {
                *self.current_tool.borrow_mut() = CurrentDrawingTool::Select;
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
            _ if hit(keys.redo) => self.redo(),
            _ if hit(keys.undo) => self.undo(),
            _ if hit(keys.clear) => self.clear(),
            // Before copy and save, which ignore Shift: with some keymaps Ctrl+Shift+S
            // arrives as a lowercase `s`.
            _ if hit(keys.copy_region) => self.capture(Capture::Copy, true),
            _ if hit(keys.save_region) => self.capture(Capture::Save, true),
            _ if hit(keys.copy) => self.capture(Capture::Copy, false),
            _ if hit(keys.save) => self.capture(Capture::Save, false),
            _ if hit(keys.quit) => self.app.quit(),
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
                *state.current_tool.borrow_mut() = tool;
                state.redraw();
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
                    *width = (*width + delta).clamp(config::MIN_LINE_WIDTH, config::MAX_LINE_WIDTH);
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

        toolbar.connect_redo(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move || state.redo(),
        ));

        toolbar.connect_clear(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move || state.clear(),
        ));

        toolbar.connect_copy_region(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move || state.capture(Capture::Copy, true),
        ));

        toolbar.connect_quit(glib::clone!(
            #[strong(rename_to = state)]
            self,
            move || state.app.quit(),
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

    let keys = keybinds::Keybinds::from_config(&conf);
    let toolbar = toolbar::Toolbar::new();
    toolbar.update(&CurrentDrawingTool::NormalLine, &colors::RED, line_width);
    toolbar.show_keybinds(&keys);

    let state = State {
        app: application.clone(),
        keybinds: Rc::new(Cell::new(keys)),
        conf: Rc::new(RefCell::new(conf)),
        config_monitor: Rc::new(RefCell::new(None)),
        elements: Rc::new(RefCell::new(history::History::new())),
        color: Rc::new(RefCell::new(colors::RED)),
        current_tool: Rc::new(RefCell::new(CurrentDrawingTool::NormalLine)),
        line_width: Rc::new(RefCell::new(line_width)),
        text_input_mode: Rc::new(RefCell::new(false)),
        shift_held: Rc::new(RefCell::new(false)),
        toolbar: Rc::new(toolbar),
        canvases: Rc::new(RefCell::new(Vec::new())),
        next_canvas_id: Rc::new(Cell::new(0)),
        capturing: Rc::new(Cell::new(false)),
        selecting_region: Rc::new(Cell::new(false)),
        pointer: Rc::new(Cell::new(None)),
        drag: Rc::new(Cell::new(None)),
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

/// The application ID: GApplication uniqueness (a second launch reaching the running
/// instance) and desktop notifications go by it.
const APP_ID: &str = "io.github.nikolatzotchev.Chicolli";

fn main() {
    let application = gtk::Application::new(Some(APP_ID), Default::default());

    application.add_main_option(
        "version",
        glib::Char::from(b'V'),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Print the version and exit",
        None,
    );
    application.connect_handle_local_options(|_, options| {
        if options.contains("version") {
            println!("chicolli {}", env!("CARGO_PKG_VERSION"));
            return std::ops::ControlFlow::Break(glib::ExitCode::SUCCESS);
        }
        std::ops::ControlFlow::Continue(())
    });

    application.connect_activate(|app| {
        // A second launch only re-activates the existing overlay (see `activate`).
        if app.windows().is_empty() {
            activate(app);
        }
    });

    application.run();
}
