use std::{rc::Rc, cell::RefCell};
use drawing::drawing_tool::DrawingTool;
use gtk4 as gtk;
use gio::{prelude::*, glib};
use gtk::{prelude::*, gdk::Display, Inhibit};

pub mod drawing;
pub mod colors;
// https://github.com/wmww/gtk-layer-shell/blob/master/examples/simple-example.c
fn activate(application: &gtk::Application) {
    // Create a normal GTK window however you like
    let window = gtk::ApplicationWindow::new(application);

    // Before the window is first realized, set it up to be a layer surface
    gtk4_layer_shell::init_for_window(&window);
    // gtk4_layer_shell::set_keyboard_mode(&window, gtk4_layer_shell::KeyboardMode::OnDemand);
    gtk4_layer_shell::set_keyboard_mode(&window, gtk4_layer_shell::KeyboardMode::OnDemand);
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
    let elements: Rc<RefCell<Vec<Box<dyn DrawingTool>>>> = Rc::new(RefCell::new(Vec::new()));

    let color = Rc::new(RefCell::new(colors::RED));
   
    let current_tool = Rc::new(RefCell::new(drawing::drawing_tool::CurrentDrawingTool::NormalLine));

    let key_controller = gtk::EventControllerKey::new();

    key_controller.connect_key_pressed(glib::clone!(@strong color, @strong current_tool => @default-return gtk::Inhibit(false), move |_, keyval, key_number , _| {
        println!( "Key pressed: keyval={}", keyval);
        println!( "Key number = {}", key_number);
        match key_number {
            // COLORS
            // r for red
            27 => *color.borrow_mut() = colors::RED,
            // g for green 
            42 => *color.borrow_mut() = colors::GREEN,
            // b for blue
            56 => *color.borrow_mut() = colors::BLUE,
            // can add more later
            // TOOLS
            // keyboard key 1
            10 => *current_tool.borrow_mut() = drawing::drawing_tool::CurrentDrawingTool::NormalLine,
            // keyboard key 2
            11 => *current_tool.borrow_mut() = drawing::drawing_tool::CurrentDrawingTool::NormalArrowHeadBase,
            // keyboard key 3
            12 => *current_tool.borrow_mut() = drawing::drawing_tool::CurrentDrawingTool::NormalArrowHeadPointer,
            // keyboard key 4
            13 => *current_tool.borrow_mut() = drawing::drawing_tool::CurrentDrawingTool::NormalRectangle,
            _ => ()
        };
        gtk::Inhibit(false)
    }));

    // key controller is added to the window and not to the drawarea because there it does not
    // work
    window.add_controller(key_controller); 

    // Set up a widget
    let draw = gtk::DrawingArea::new();

    let motion_controller = gtk::EventControllerMotion::new();
    motion_controller.connect_motion(glib::clone!(@weak draw, @strong elements => move |_, x, y| {
        if let Some(elem) = elements.borrow_mut().last_mut() {
            elem.motion_notify(drawing::drawing_tool::Point(x, y));
            if elem.active() {
                 draw.queue_draw();
            }
        }
    }));

    draw.add_controller(motion_controller);

      
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

    let left_click_mouse = gtk::GestureClick::new();

    // Set the gestures button to the right mouse button (=3)
    left_click_mouse.set_button(gtk::gdk::ffi::GDK_BUTTON_PRIMARY as u32);
   
    // Assign your handler to an event of the gesture (e.g. the `pressed` event)
    left_click_mouse.connect_pressed(glib::clone!(@strong elements, @strong current_tool, @strong line_width => move |_, _, x, y| {
        let mut drawing_tool: Box<dyn drawing::drawing_tool::DrawingTool> = match *current_tool.borrow() {
            drawing::drawing_tool::CurrentDrawingTool::NormalLine => Box::new(drawing::normal_line::NormalLine::new()),
            drawing::drawing_tool::CurrentDrawingTool::NormalArrowHeadBase => Box::new(drawing::arrow::NormalArrow::new(true)),
            drawing::drawing_tool::CurrentDrawingTool::NormalArrowHeadPointer => Box::new(drawing::arrow::NormalArrow::new(false)),
            drawing::drawing_tool::CurrentDrawingTool::NormalRectangle => Box::new(drawing::normal_rectangle::NormalRectangle::new()),
        };
        drawing_tool.press_mouse(drawing::drawing_tool::Point(x, y));
        drawing_tool.set_line_width(*line_width.borrow());
        drawing_tool.set_color(*color.borrow());
        elements.borrow_mut().push(drawing_tool);
    }));

    left_click_mouse.connect_released(glib::clone!(@strong elements => move |_, _, x, y| {
        if let Some(elem) = elements.borrow_mut().last_mut() {
            elem.release_mouse(drawing::drawing_tool::Point(x, y));
        }
    }));

    draw.add_controller(left_click_mouse);
 
    // scroll controller 
    let scroll_controller = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
    
    scroll_controller.connect_scroll(glib::clone!(@strong line_width => @default-return Inhibit(false), move |_, _,  scroll| {

        let mut width = line_width.borrow_mut();
        let new_width = *width - scroll;
        if new_width as i32 >= 1 {
            *width = new_width;
        } else {
            *width = 1.0;
        }

        Inhibit(false)
    }));

    draw.add_controller(scroll_controller);

    draw.set_draw_func(glib::clone!(@weak elements => move |_, ctx, _, _| {

        for element in elements.borrow_mut().iter() {
            element.draw(ctx); 
        }

        if let Err(error) = ctx.fill() { 
            panic!("error drawing: {:?}", error)
        };
    }));

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
