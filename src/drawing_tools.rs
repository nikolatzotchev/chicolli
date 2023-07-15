use gtk4::cairo::Context;

#[derive(Clone, Debug, Copy)]
pub struct Point(pub f64,pub f64);

impl std::ops::Add<Point> for Point {
    type Output = Point;

    fn add(self, rhs: Point) -> Self::Output {
        Point(self.0 + rhs.0, self.1 + rhs.1)
    }
}
impl std::ops::Sub<Point> for Point {
    type Output = Point;

    fn sub(self, rhs: Point) -> Self::Output {
        Point(self.0 - rhs.0, self.1 - rhs.1)
    }
}
impl std::ops::Mul<f64> for Point {
   type Output = Point;

   fn mul(self, rhs: f64) -> Self::Output {
       Point(self.0 * rhs, self.1 * rhs)
   } 
}

impl std::ops::Div<f64> for Point {
   type Output = Point;

   fn div(self, rhs: f64) -> Self::Output {
       Point(self.0 / rhs, self.1 / rhs)
   } 
}

fn subtract_points(p1: Point, p2:Point) -> Point {
    return Point(p1.0 - p2.0, p1.1 - p2.1);
}

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

pub fn calc_centripetal_catmullrom_spline(p0: &Point, p1: &Point, p2: &Point, p3: &Point) -> (Point, Point) {

    let d_0 = (*p1 - *p0) / 3.0;
    let d_3 = (*p3 - *p2) / 3.0;
    let b_1 = -0.25;
    let b_2 = -1.0 / (4.0 + b_1);
    let a_1 = (*p2 - *p0 - d_0) / 4.0;
    let a_2 = (*p3 - *p1 - a_1) / (4.0 + b_1);
    
    let d_2 = a_2 + d_3 * b_2;
    let d_1 = a_1 + d_2 * b_1 ;
    (*p1+d_1, *p2-d_2)
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
            ctx.set_line_join(gtk4::cairo::LineJoin::Round);
            ctx.move_to(last.0, last.1);
            
            for chunk in self.points.windows(4) {
                match chunk {
                    [p1, p2, p3, p4] => {
                        let (f1, f2) = calc_centripetal_catmullrom_spline(p1,p2,p3,p4);
                        ctx.curve_to(f1.0, f1.1,
                                     f2.0, f2.1,
                                     p3.0, p3.1);
                    },
                    _ => unreachable!(),
                }
            }

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
