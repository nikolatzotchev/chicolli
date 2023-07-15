use gtk4::cairo::Context;

#[derive(Clone, Debug)]
pub struct Point(pub f64,pub f64);

pub enum CurrentDrawingTool {
    NormalLine,
    NormalArrow,
    NormalRectangle
}

pub trait DrawingTool {
    fn release_mouse(&mut self, point: Point);
    fn press_mouse(&mut self, point: Point);
    fn motion_notify(&mut self, point: Point);
    fn draw(&self, cnx: &Context);
    fn set_line_width(&mut self, width: f64);
}

pub struct NormalLine {
    points: Vec<Point>,
    finished: bool,
    started: bool,
    line_width: f64
}

impl NormalLine {
    pub fn new() -> NormalLine {
        NormalLine {
            points: Vec::new(),
            finished: false,
            started: false,
            line_width: 2.0 
        }
    }
}

impl DrawingTool for NormalLine {
    fn release_mouse(&mut self, _: Point) {
        self.finished = true;
    }

    fn press_mouse(&mut self, _: Point) {
        self.started = true;
    }

    fn motion_notify(&mut self, point: Point) {
        if self.started && !self.finished {
            self.points.push(point);
        }
    }

    fn draw(&self, ctx: &Context) -> () {
        ctx.set_line_width(self.line_width);
        if let Some(last) = self.points.first() {
            // this makes corners round 
            ctx.set_line_cap(gtk4::cairo::LineCap::Round); 
            ctx.move_to(last.0, last.1);
            let mut prev = last;
            for chunk in self.points.chunks(3) {
                match chunk {
                    [p1] => ctx.line_to(p1.0, p1.1),
                    [p1, p2] => ctx.curve_to(p1.0, p1.1, p1.0, p1.1, p2.0, p2.1),
                    [p1, p2, p3] => {
                        let mut first_bezier = p1.clone();
                        let mut second_bezier = p2.clone();
                        if p1.0 > prev.0 {
                            first_bezier.0 += 10.0;
                        } else {
                            first_bezier.0 -= 10.0;
                        }
                        if p1.1 > prev.1 {
                            first_bezier.1 += 10.0;
                        } else {
                            first_bezier.1 -= 10.0;
                        }

                        if p2.0 > p3.0 {
                            second_bezier.0 += 10.0;
                        } else {
                            second_bezier.0 -= 10.0;
                        }
                        if p2.1 > p3.1  {
                            second_bezier.1 += 10.0;
                        } else {
                            second_bezier.1 -= 10.0;
                        }
                        ctx.curve_to(first_bezier.0, first_bezier.1, second_bezier.0, second_bezier.1, p3.0, p3.1)
                    },
                    _ => unreachable!()
                }
                if let Some(a) = chunk.last() {
                    prev = a;
                }
            }
            ctx.set_line_join(gtk4::cairo::LineJoin::Round);

            match ctx.stroke() {
                Err(e) => println!("{e}"),
                _ => ()
            }
        }
    }

    fn set_line_width(&mut self, width: f64) {
        self.line_width = width;
    } 
}
/*
 pub struct NormalRectangle {
    start: Option<(f64, f64)>,
    end: Option<(f64, f64)>,
    started: bool,
    finished: bool,
}

impl NormalRectangle {
    pub fn new() -> NormalRectangle {
        NormalRectangle { start: None, end: None, started: false, finished: false }
    }
}

impl DrawingTool for NormalRectangle {

    fn release_mouse(&mut self, event: &EventButton) {
        self.finished = true;
        self.end = Some(event.position());
    }

    fn press_mouse(&mut self, event: &EventButton) {
        self.started = true;
        self.start = Some(event.position());
    }

    fn motion_notify(&mut self, da: &DrawingArea, event: &EventMotion) {
        if !self.finished {
            self.end = Some(event.position());
            da.queue_draw();
        }
    }

    fn draw(&self, ctx: &Context) {
        if self.started {
            
            ctx.set_line_width(2.0);

            if let (Some(start), Some(end)) = (self.start, self.end) {
                ctx.rectangle(
                    f64::min(start.0, end.0), 
                    f64::min(start.1, end.1), 
                    (end.0 - start.0).abs(),
                    (end.1 - start.1).abs());
            } 
           
            match ctx.stroke() {
                Err(e) => println!("{e}"),
                _ => ()
            }
        }
    }
}

pub struct NormalArrow {
    start: Option<(f64, f64)>,
    end: Option<(f64, f64)>,
    arrow_length: f64,
    arrow_degree: f64,
    started: bool,
    finished: bool,
}

impl NormalArrow {
    pub fn new() -> NormalArrow {
        NormalArrow {
            start: None,
            end: None,
            arrow_length: 20.0,
            started: false,
            finished: false,
            arrow_degree: 0.58067840828
        }
    }
}

impl DrawingTool for NormalArrow {

    fn release_mouse(&mut self, event: &EventButton) {
        self.finished = true;
        self.end = Some(event.position());
    }

    fn press_mouse(&mut self, event: &EventButton) {
        self.started = true;
        self.start = Some(event.position());
    }

    fn motion_notify(&mut self, da: &DrawingArea , event: &EventMotion) {
        if !self.finished {
            self.end = Some(event.position());
            da.queue_draw();
        }
    }

    fn draw(&self, ctx: &Context) {
        if self.started {
            ctx.set_line_width(2.0);
            if let (Some(start), Some(end)) = (self.start, self.end) {

                ctx.move_to(start.0, start.1);
                ctx.line_to(end.0, end.1);
                
                let angle_main_line = (end.1 - start.1).atan2(end.0 - start.0);

                let (x1, y1): (f64, f64);

                let (x2, y2): (f64, f64);

                x1 = start.0 + self.arrow_length * (angle_main_line - self.arrow_degree).cos();
                y1 = start.1 + self.arrow_length * (angle_main_line - self.arrow_degree).sin();
                x2 = start.0 + self.arrow_length * (angle_main_line + self.arrow_degree).cos();
                y2 = start.1 + self.arrow_length * (angle_main_line + self.arrow_degree).sin();

                ctx.move_to(start.0, start.1);
                ctx.line_to(x1, y1);

                ctx.move_to(start.0, start.1);
                ctx.line_to(x2, y2);

                match ctx.stroke() {
                    Err(e) => println!("{e}"),
                    _ => ()
                }
            }
        }

    }
}
*/
