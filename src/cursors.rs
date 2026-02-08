use gtk::gdk;
use gtk::glib;

const SIZE: i32 = 32;
const OUTLINE: f64 = 2.5;

fn surface_to_cursor(surface: &mut gtk::cairo::ImageSurface, hotspot_x: i32, hotspot_y: i32) -> Option<gdk::Cursor> {
    surface.flush();
    let bytes = glib::Bytes::from(surface.data().ok()?.as_ref());
    let texture = gdk::MemoryTexture::new(
        SIZE,
        SIZE,
        gdk::MemoryFormat::B8g8r8a8Premultiplied,
        &bytes,
        (SIZE * 4) as usize,
    );
    Some(gdk::Cursor::from_texture(
        &texture,
        hotspot_x,
        hotspot_y,
        None,
    ))
}

fn pencil_path(cr: &gtk::cairo::Context) {
    cr.save().unwrap();
    cr.translate(5.0, 27.0);
    cr.rotate(-std::f64::consts::FRAC_PI_4);

    cr.rectangle(0.0, -4.0, 22.0, 8.0);
    cr.move_to(-5.0, 0.0);
    cr.line_to(0.0, -4.0);
    cr.line_to(0.0, 4.0);
    cr.close_path();
    cr.rectangle(22.0, -4.0, 5.0, 8.0);

    cr.restore().unwrap();
}

pub fn pencil_cursor() -> Option<gdk::Cursor> {
    let mut surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, SIZE, SIZE).ok()?;
    {
        let cr = gtk::cairo::Context::new(&surface).ok()?;
        cr.set_antialias(gtk::cairo::Antialias::Best);

        // White outline pass
        pencil_path(&cr);
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.set_line_width(OUTLINE + 1.5);
        cr.set_line_join(gtk::cairo::LineJoin::Round);
        cr.stroke().ok()?;

        // Filled shapes
        cr.save().ok()?;
        cr.translate(5.0, 27.0);
        cr.rotate(-std::f64::consts::FRAC_PI_4);

        cr.set_source_rgb(0.95, 0.85, 0.2);
        cr.rectangle(0.0, -4.0, 22.0, 8.0);
        cr.fill().ok()?;

        cr.set_source_rgb(0.4, 0.4, 0.4);
        cr.move_to(-5.0, 0.0);
        cr.line_to(0.0, -4.0);
        cr.line_to(0.0, 4.0);
        cr.close_path();
        cr.fill().ok()?;

        cr.set_source_rgb(0.9, 0.5, 0.5);
        cr.rectangle(22.0, -4.0, 5.0, 8.0);
        cr.fill().ok()?;

        // Dark inner outlines
        cr.set_source_rgb(0.0, 0.0, 0.0);
        cr.set_line_width(1.0);
        cr.rectangle(0.0, -4.0, 22.0, 8.0);
        cr.stroke().ok()?;
        cr.move_to(-5.0, 0.0);
        cr.line_to(0.0, -4.0);
        cr.line_to(0.0, 4.0);
        cr.close_path();
        cr.stroke().ok()?;
        cr.rectangle(22.0, -4.0, 5.0, 8.0);
        cr.stroke().ok()?;

        cr.restore().ok()?;
    }
    surface_to_cursor(&mut surface, 2, 30)
}

fn arrow_path(cr: &gtk::cairo::Context, x1: f64, y1: f64, x2: f64, y2: f64, head_len: f64, a1: f64, a2: f64) {
    cr.move_to(x1, y1);
    cr.line_to(x2, y2);
    cr.move_to(x2, y2);
    cr.line_to(x2 - head_len * a1.cos(), y2 - head_len * a1.sin());
    cr.move_to(x2, y2);
    cr.line_to(x2 - head_len * a2.cos(), y2 - head_len * a2.sin());
}

pub fn arrow_cursor() -> Option<gdk::Cursor> {
    let mut surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, SIZE, SIZE).ok()?;
    {
        let cr = gtk::cairo::Context::new(&surface).ok()?;
        cr.set_antialias(gtk::cairo::Antialias::Best);

        let x1: f64 = 26.0;
        let y1: f64 = 6.0;
        let x2: f64 = 4.0;
        let y2: f64 = 28.0;

        let head_len: f64 = 10.0;
        let angle = (y2 - y1).atan2(x2 - x1);
        let a1 = angle + 0.45;
        let a2 = angle - 0.45;

        // White outline
        arrow_path(&cr, x1, y1, x2, y2, head_len, a1, a2);
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.set_line_width(OUTLINE + 2.5);
        cr.set_line_cap(gtk::cairo::LineCap::Round);
        cr.set_line_join(gtk::cairo::LineJoin::Round);
        cr.stroke().ok()?;

        // Colored shaft and head
        arrow_path(&cr, x1, y1, x2, y2, head_len, a1, a2);
        cr.set_source_rgb(0.9, 0.3, 0.1);
        cr.set_line_width(2.5);
        cr.stroke().ok()?;

        // Filled arrowhead triangle
        cr.move_to(x2, y2);
        cr.line_to(x2 - head_len * a1.cos(), y2 - head_len * a1.sin());
        cr.line_to(x2 - head_len * a2.cos(), y2 - head_len * a2.sin());
        cr.close_path();
        cr.set_source_rgb(0.9, 0.3, 0.1);
        cr.fill().ok()?;
    }
    surface_to_cursor(&mut surface, 4, 28)
}

fn crosshair_path(cr: &gtk::cairo::Context) {
    cr.move_to(6.0, 16.0);
    cr.line_to(26.0, 16.0);
    cr.move_to(16.0, 6.0);
    cr.line_to(16.0, 26.0);
}

pub fn rectangle_cursor() -> Option<gdk::Cursor> {
    let mut surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, SIZE, SIZE).ok()?;
    {
        let cr = gtk::cairo::Context::new(&surface).ok()?;
        cr.set_antialias(gtk::cairo::Antialias::Best);

        // White outline for crosshair
        crosshair_path(&cr);
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.set_line_width(OUTLINE + 1.5);
        cr.set_line_cap(gtk::cairo::LineCap::Round);
        cr.stroke().ok()?;

        // White outline for rectangle preview
        cr.rectangle(16.0, 16.0, 12.0, 10.0);
        cr.set_line_width(OUTLINE + 1.5);
        cr.stroke().ok()?;

        // Black crosshair
        crosshair_path(&cr);
        cr.set_source_rgb(0.0, 0.0, 0.0);
        cr.set_line_width(1.5);
        cr.stroke().ok()?;

        // Blue rectangle preview
        cr.set_source_rgba(0.2, 0.6, 1.0, 0.4);
        cr.rectangle(16.0, 16.0, 12.0, 10.0);
        cr.fill().ok()?;

        cr.set_source_rgb(0.2, 0.6, 1.0);
        cr.set_line_width(1.5);
        cr.rectangle(16.0, 16.0, 12.0, 10.0);
        cr.stroke().ok()?;
    }
    surface_to_cursor(&mut surface, 16, 16)
}

fn ibeam_path(cr: &gtk::cairo::Context) {
    cr.move_to(10.0, 4.0);
    cr.line_to(22.0, 4.0);
    cr.move_to(16.0, 4.0);
    cr.line_to(16.0, 28.0);
    cr.move_to(10.0, 28.0);
    cr.line_to(22.0, 28.0);
}

pub fn text_cursor() -> Option<gdk::Cursor> {
    let mut surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, SIZE, SIZE).ok()?;
    {
        let cr = gtk::cairo::Context::new(&surface).ok()?;
        cr.set_antialias(gtk::cairo::Antialias::Best);

        // White outline
        ibeam_path(&cr);
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.set_line_width(OUTLINE + 2.0);
        cr.set_line_cap(gtk::cairo::LineCap::Round);
        cr.set_line_join(gtk::cairo::LineJoin::Round);
        cr.stroke().ok()?;

        // Black I-beam
        ibeam_path(&cr);
        cr.set_source_rgb(0.0, 0.0, 0.0);
        cr.set_line_width(2.0);
        cr.stroke().ok()?;
    }
    surface_to_cursor(&mut surface, 16, 16)
}

fn highlighter_path(cr: &gtk::cairo::Context) {
    cr.save().unwrap();
    cr.translate(4.0, 28.0);
    cr.rotate(-std::f64::consts::FRAC_PI_4);

    cr.move_to(-3.0, -5.0);
    cr.line_to(3.0, -5.0);
    cr.line_to(3.0, 5.0);
    cr.line_to(-3.0, 5.0);
    cr.close_path();
    cr.rectangle(3.0, -5.0, 20.0, 10.0);
    cr.rectangle(23.0, -5.0, 5.0, 10.0);

    cr.restore().unwrap();
}

pub fn highlighter_cursor() -> Option<gdk::Cursor> {
    let mut surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, SIZE, SIZE).ok()?;
    {
        let cr = gtk::cairo::Context::new(&surface).ok()?;
        cr.set_antialias(gtk::cairo::Antialias::Best);

        // White outline pass
        highlighter_path(&cr);
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.set_line_width(OUTLINE + 1.5);
        cr.set_line_join(gtk::cairo::LineJoin::Round);
        cr.stroke().ok()?;

        // Filled shapes
        cr.save().ok()?;
        cr.translate(4.0, 28.0);
        cr.rotate(-std::f64::consts::FRAC_PI_4);

        cr.set_source_rgba(1.0, 1.0, 0.0, 0.8);
        cr.move_to(-3.0, -5.0);
        cr.line_to(3.0, -5.0);
        cr.line_to(3.0, 5.0);
        cr.line_to(-3.0, 5.0);
        cr.close_path();
        cr.fill().ok()?;

        cr.set_source_rgba(1.0, 0.85, 0.0, 0.95);
        cr.rectangle(3.0, -5.0, 20.0, 10.0);
        cr.fill().ok()?;

        cr.set_source_rgb(0.6, 0.5, 0.0);
        cr.rectangle(23.0, -5.0, 5.0, 10.0);
        cr.fill().ok()?;

        // Dark inner outlines
        cr.set_source_rgb(0.0, 0.0, 0.0);
        cr.set_line_width(1.0);
        cr.move_to(-3.0, -5.0);
        cr.line_to(3.0, -5.0);
        cr.line_to(3.0, 5.0);
        cr.line_to(-3.0, 5.0);
        cr.close_path();
        cr.stroke().ok()?;
        cr.rectangle(3.0, -5.0, 20.0, 10.0);
        cr.stroke().ok()?;
        cr.rectangle(23.0, -5.0, 5.0, 10.0);
        cr.stroke().ok()?;

        cr.restore().ok()?;
    }
    surface_to_cursor(&mut surface, 2, 30)
}
