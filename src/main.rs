use std::{rc::Rc, cell::RefCell};

use gtk4 as gtk;
use gio::{prelude::*, glib};
use gtk::{prelude::*, gdk::Display, Inhibit};

mod drawing_tools;
pub mod colors;
// https://github.com/wmww/gtk-layer-shell/blob/master/examples/simple-example.c
fn activate(application: &gtk::Application) {
    // Create a normal GTK window however you like
    let window = gtk::ApplicationWindow::new(application);

    // Before the window is first realized, set it up to be a layer surface
    gtk4_layer_shell::init_for_window(&window);
    gtk4_layer_shell::set_keyboard_mode(&window, gtk4_layer_shell::KeyboardMode::Exclusive);
    // Display above normal windows
    gtk4_layer_shell::set_layer(&window, gtk4_layer_shell::Layer::Overlay);

    // Anchors are if the window is pinned to each edge of the output
    let anchors = [
        (gtk4_layer_shell::Edge::Left, true),
        (gtk4_layer_shell::Edge::Right, true),
        (gtk4_layer_shell::Edge::Top, true),
        (gtk4_layer_shell::Edge::Bottom, true),
    ];

    for (anchor, state) in anchors {
        gtk4_layer_shell::set_anchor(&window, anchor, state);
    }

    // main components
    let elements: Rc<RefCell<Vec<Box<dyn drawing_tools::DrawingTool>>>> = Rc::new(RefCell::new(Vec::new()));
    let elements_draw_copy = elements.clone();
    let elements_mouse_1_press_copy = elements.clone();
    let elements_mouse_2_press_copy = elements.clone();
    let elements_motion_copy = elements.clone();

    let color = Rc::new(RefCell::new(colors::RED));
    let color_copy_change = color.clone();

    // Set up a widget
    let draw = gtk::DrawingArea::new();

    let motion_controller =  gtk::EventControllerMotion::new();
    motion_controller.connect_motion(glib::clone!(@weak draw => move |_, x, y| {
        if let Some(elem) = elements_motion_copy.borrow_mut().last_mut() {
            elem.motion_notify(drawing_tools::Point(x, y));
            if elem.active() {
                 draw.queue_draw();
            }
        }
    }));

    draw.add_controller(motion_controller);

    let key_controller = gtk::EventControllerKey::new();

    key_controller.connect_key_pressed(move |_, keyval, key_number , _| {
        println!( "Key pressed: keyval={}", keyval);
        println!( "Key number = {}", key_number);
        match key_number {
            // r for red
            27 => *color_copy_change.borrow_mut() = colors::RED,
            // g for green 
            42 => *color_copy_change.borrow_mut() = colors::GREEN,
            // b for blue
            56 => *color_copy_change.borrow_mut() = colors::BLUE,
            // can add more later
            _ => ()
        };
        gtk::Inhibit(false)
    });

    // key controller is added to the window and not to the drawarea because there it does not
    // work
    window.add_controller(key_controller); 
   
    let right_click_mouse = gtk::GestureClick::new();

    // Set the gestures button to the right mouse button (=3)
    right_click_mouse.set_button(gtk::gdk::ffi::GDK_BUTTON_SECONDARY as u32);
   
    // Assign your handler to an event of the gesture (e.g. the `pressed` event)
    right_click_mouse.connect_pressed(|_, _, _, _| {
        // exit the application
        std::process::exit(1);
    });
    
    draw.add_controller(right_click_mouse);
    
    let line_width = Rc::new(RefCell::new(2.0));
    let line_width_draw_copy = line_width.clone();
    let line_width_scrool_copy = line_width.clone();

    let left_click_mouse = gtk::GestureClick::new();

    // Set the gestures button to the right mouse button (=3)
    left_click_mouse.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
   
    // Assign your handler to an event of the gesture (e.g. the `pressed` event)
    left_click_mouse.connect_pressed(move |_, _, x, y| {
        let mut drawing_tool: Box<dyn drawing_tools::DrawingTool> = Box::new(drawing_tools::NormalLine::new());
        drawing_tool.press_mouse(drawing_tools::Point(x, y));
        drawing_tool.set_line_width(*line_width_draw_copy.borrow());
        drawing_tool.set_color(*color.borrow());
        elements_mouse_1_press_copy.borrow_mut().push(drawing_tool);
    });

    left_click_mouse.connect_released(move |_, _, x, y| {
        if let Some(elem) = elements_mouse_2_press_copy.borrow_mut().last_mut() {
            elem.release_mouse(drawing_tools::Point(x, y));
        }
    });

    draw.add_controller(left_click_mouse);
 
    // scrool controller 
    let scrool_controller = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
    
    scrool_controller.connect_scroll(move |_, _,  scrool| {
        let p:i32 = scrool as i32;
        let mut width = line_width_scrool_copy.borrow_mut();

        match p {
            1 => {
                if *width > 1.0 {
                    *width -= 1.0;
                }
            }, 
            -1 =>  *width += 1.0,
            _ => ()

        }
        Inhibit(false)
    });

    draw.add_controller(scrool_controller);

    draw.set_draw_func(move |_, ctx, _, _| {

        for element in elements_draw_copy.borrow_mut().iter() {
            element.draw(ctx); 
        }

        if let Err(error) = ctx.fill() { 
            panic!("error drawing: {:?}", error)
        };
    });

    // load css for the transparency of the window
    let provider = gtk::CssProvider::new();
    provider.load_from_data(include_str!("styles/style.css"));
    gtk::style_context_add_provider_for_display(&Display::default().expect("error getting default display"), &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

    window.set_child(Some(&draw));
    window.show();
  }

fn main() {
    let application = gtk::Application::new(Some("sh.wmww.gtk-layer-example"), Default::default());

    application.connect_activate(|app| {
        activate(app);
    });

    application.run();
}
