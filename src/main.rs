use drawing::drawing_tool::DrawingTool;

use gio::Cancellable;
use gtk::glib::{self, Propagation};
use gtk::{
    cairo::Region,
    gdk::{Display, Key},
    prelude::*,
};

use std::{cell::RefCell, rc::Rc};

pub mod colors;
pub mod config;
pub mod drawing;

// https://github.com/wmww/gtk-layer-shell/blob/master/examples/simple-example.c
fn activate(application: &gtk::Application) {
    // Create a normal GTK window however you like
    let window = gtk::ApplicationWindow::new(application);

    application.connect_activate(glib::clone!(@weak window => move |_| {
        gtk4_layer_shell::set_keyboard_mode(&window, gtk4_layer_shell::KeyboardMode::Exclusive);
        window.surface().set_opaque_region(Some(&Region::create()));
    }));

    let conf = Rc::new(config::get_config());

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
    let elements: Rc<RefCell<Vec<Box<dyn DrawingTool>>>> = Rc::new(RefCell::new(Vec::new()));

    let color = Rc::new(RefCell::new(colors::RED));

    let current_tool = Rc::new(RefCell::new(
        drawing::drawing_tool::CurrentDrawingTool::NormalLine,
    ));

    let key_controller = gtk::EventControllerKey::new();

    let color_dialog = Rc::new(
        gtk::ColorDialog::builder()
            .title("Choose color")
            .modal(true)
            .build(),
    );
   
    

    // get the tools cursors
    let mut pencil_cur = gtk::gdk::Cursor::from_name("default", None);
    let mut arrow_cur = gtk::gdk::Cursor::from_name("default", None);
    let mut rectangle_cur = gtk::gdk::Cursor::from_name("default", None);

    let cursors_loc = config::get_cursors_config_loc();
    if let Some(curs_loc) = cursors_loc {
        if curs_loc.as_path().exists() {
            let paths = std::fs::read_dir(curs_loc.as_path());
            if let Ok(paths) = paths {
                for path in paths {
                    if let Ok(path) = path {
                        if let Some(file_name) = path.path().file_stem() {
                            if let Some(file_name) = file_name.to_str() {
                                let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_file_at_scale(path.path(), 30, 30, true).unwrap();
                                let cur_texture = gtk::gdk::Texture::for_pixbuf(&pixbuf);
                                let cur = Some(gtk::gdk::Cursor::from_texture(&cur_texture, 0, 0, None));

                                match file_name {
                                    config::PENCIL_CUR => {
                                        pencil_cur = cur;
                                    },
                                    config::ARROW_CUR => {
                                        arrow_cur = cur;
                                    },
                                    config::SQUARE_CUR =>  {
                                        rectangle_cur = cur;
                                    },
                                    _ => (),
                                }
                            }
                        }
                    }
                }
            }
        } 
    }
   
    // Set up a widget
    let draw = gtk::DrawingArea::new();
    // the default cursor should be the pencil one
    if let Some(pencil_cur) = pencil_cur.clone() {
        draw.set_cursor(Some(&pencil_cur));
    } 

    key_controller.connect_key_pressed(glib::clone!(@strong draw, @strong window as w, @strong color_dialog, @strong conf, @strong color, @strong current_tool => @default-return Propagation::Proceed, move |_, keyval, _, _| {
        // close your eyes 
        let _draw_key = Key::from_name(conf.draw_keybind.as_deref().unwrap_or("")).unwrap_or(Key::Abelowdot);
        let _arrow_key = Key::from_name(conf.arrow_keybind.as_deref().unwrap_or("")).unwrap_or(Key::Abelowdot);
        let _reverse_arrow_key = Key::from_name(conf.reverse_arrow_keybind.as_deref().unwrap_or("")).unwrap_or(Key::Abelowdot);
        let _rectangle_key = Key::from_name(conf.rectangle_keybind.as_deref().unwrap_or("")).unwrap_or(Key::Abelowdot);
        let _disable_drawing_key = Key::from_name(conf.disable_drawing.as_deref().unwrap_or("")).unwrap_or(Key::Abelowdot);
        let _color_r = Key::from_name(conf.color_r.as_deref().unwrap_or("")).unwrap_or(Key::Abelowdot);
        let _color_g = Key::from_name(conf.color_g.as_deref().unwrap_or("")).unwrap_or(Key::Abelowdot);
        let _color_b = Key::from_name(conf.color_b.as_deref().unwrap_or("")).unwrap_or(Key::Abelowdot);
        let _color_chooser = Key::from_name(conf.color_chooser.as_deref().unwrap_or("")).unwrap_or(Key::Abelowdot);

        match keyval {
            // TOOLS
            _ if _draw_key == keyval => { 
                *current_tool.borrow_mut() = drawing::drawing_tool::CurrentDrawingTool::NormalLine;
                if let Some(pencil_cur) = pencil_cur.clone() {
                    draw.set_cursor(Some(&pencil_cur));
                }
            },
            _ if _arrow_key == keyval => {
                *current_tool.borrow_mut() = drawing::drawing_tool::CurrentDrawingTool::NormalArrowHeadPointer;
                if let Some(arrow_cur) = arrow_cur.clone() {
                    draw.set_cursor(Some(&arrow_cur));
                }
            }  
            _ if _reverse_arrow_key == keyval => {
                *current_tool.borrow_mut() = drawing::drawing_tool::CurrentDrawingTool::NormalArrowHeadBase;
                if let Some(arrow_cur) = arrow_cur.clone() {
                    draw.set_cursor(Some(&arrow_cur));
                }
            }  
            _ if _rectangle_key == keyval => {
                *current_tool.borrow_mut() = drawing::drawing_tool::CurrentDrawingTool::NormalRectangle;
                if let Some(rectangle_cur) = rectangle_cur.clone() {
                    draw.set_cursor(Some(&rectangle_cur));
                }

            },           
            _ if _disable_drawing_key == keyval => {
                gtk4_layer_shell::set_keyboard_mode(&w, gtk4_layer_shell::KeyboardMode::None);
                w.surface().set_input_region(&Region::create());
                w.unmap();
                w.map();
            },
            // colors
            _ if _color_r == keyval =>  *color.borrow_mut() = colors::RED,
            _ if _color_g == keyval =>  *color.borrow_mut() = colors::GREEN,
            _ if _color_b == keyval =>  *color.borrow_mut() = colors::BLUE,
            _ if _color_chooser == keyval => {
                gtk4_layer_shell::set_layer(&w, gtk4_layer_shell::Layer::Bottom);
                color_dialog.choose_rgba(
                    None::<&gtk::Window>,
                    Some(&gtk::gdk::RGBA::RED),
                    None::<&Cancellable>,
                    glib::clone!(@strong color, @weak w => move |c| match c {
                        Ok(c) => {
                            gtk4_layer_shell::set_layer(&w, gtk4_layer_shell::Layer::Overlay);
                            *color.borrow_mut() = c;
                        },
                        Err(_) => {
                            // Dismissed by user
                            gtk4_layer_shell::set_layer(&w, gtk4_layer_shell::Layer::Overlay);
                        }
                    }),
                    );
            },
            _ => (),
        };
        Propagation::Proceed
    }));

    // key controller is added to the window and not to the drawarea because there it does not
    // work
    window.add_controller(key_controller);

    let motion_controller = gtk::EventControllerMotion::new();
    motion_controller.connect_motion(
        glib::clone!(@weak draw, @strong elements => move |_, x, y| {
            if let Some(elem) = elements.borrow_mut().last_mut() {
                elem.motion_notify(drawing::drawing_tool::Point(x, y));
                if elem.active() {
                     draw.queue_draw();
                }
            }
        }),
    );

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

    let line_width = Rc::new(RefCell::new(conf.line_thickness.unwrap_or(2.0)));

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
    let scroll_controller =
        gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);

    scroll_controller.connect_scroll(
        glib::clone!(@strong line_width => @default-return Propagation::Proceed, move |_, _,  scroll| {
            let mut width = line_width.borrow_mut();
            let new_width = *width - scroll;
            if new_width as i32 >= 1 {
                *width = new_width;
            } else {
                *width = 1.0;
            }
            Propagation::Proceed
        }),
    );

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
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("error getting default display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // let ff = gtk::ColorDialog::builder().title("Choose color").build();
    // ff.choose_rgba(
    //     Some(&window),
    //     Some(&gtk::gdk::RGBA::RED),
    //     Some(&Cancellable::new()),
    //     |c| match c {
    //         Ok(c) => println!("hmm {}", c),
    //         Err(_) => (),
    //     },
    // );
    //

    // cursors
    // let pencil_bytes = include_bytes!("../resources/pencil.png");
    // let arrow_bytes = include_bytes!("../resources/arrow.png");
    // let square_bytes = include_bytes!("../resources/square.png");
    //
    // let bytes = glib::Bytes::from(pencil_bytes);
    // let texture = gtk::gdk::Texture::from_bytes(&bytes);
    // if let Ok(t) = texture {
    //     println!("{:?}", t);
    //     let arrow = gtk::gdk::Cursor::from_texture(&t, 0, 0, Some(&gtk::gdk::Cursor::from_name("crosshair", None).unwrap()));
    //     println!("{:?}", arrow);
    //     draw.set_cursor(Some(&arrow));
    // }
    //
    // let arrow = gtk::gdk::Cursor::from_name("crosshair", None);
    // if let Some(cursor) = arrow {
    //     println!("{:?}", cursor);
    //     // draw.set_cursor(Some(&gtk::gdk::Cursor::from_name("crosshair", None).unwrap()));
    //     draw.set_cursor(Some(&cursor));
    // }
    //
    window.set_child(Some(&draw));
    window.set_visible(true);
}

fn main() {
    let application = gtk::Application::new(Some("sh.wmww.gtk-layer-example"), Default::default());

    application.connect_activate(|app| {
        activate(app);
    });

    application.run();
}
