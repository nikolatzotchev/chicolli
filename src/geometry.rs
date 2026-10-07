#[derive(Clone, Debug, Copy)]
pub struct Point(pub f64, pub f64);

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

impl std::ops::Neg for Point {
    type Output = Point;

    fn neg(self) -> Point {
        Point(-self.0, -self.1)
    }
}

pub fn snap_angle(start: Point, end: Point) -> Point {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let angle = dy.atan2(dx);
    let snapped = (angle / std::f64::consts::FRAC_PI_4).round() * std::f64::consts::FRAC_PI_4;
    let len = (dx * dx + dy * dy).sqrt();
    Point(start.0 + len * snapped.cos(), start.1 + len * snapped.sin())
}

pub fn snap_square(start: Point, end: Point) -> Point {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let side = dx.abs().max(dy.abs());
    Point(start.0 + side * dx.signum(), start.1 + side * dy.signum())
}

pub fn distance_sq(a: Point, b: Point) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    dx * dx + dy * dy
}

/// Control-point offsets for an interpolating cubic spline through `points`.
///
/// Segment `i` runs from `points[i]` to `points[i + 1]` with Bezier control points
/// `points[i] + d[i]` and `points[i + 1] - d[i + 1]`. The inner offsets solve
/// `d[i - 1] + 4 d[i] + d[i + 1] = points[i + 1] - points[i - 1]`, which keeps the curve
/// C2-continuous. Returns an empty vec for fewer than 3 points.
///
/// See https://www.ibiblio.org/e-notes/Splines/b-int.html
pub fn spline_controls(points: &[Point]) -> Vec<Point> {
    let n = points.len();
    if n < 3 {
        return vec![];
    }
    let mut a = vec![Point(0.0, 0.0); n];
    let mut b = vec![0.0; n];
    let mut d = vec![Point(0.0, 0.0); n];
    d[0] = (points[1] - points[0]) / 3.0;
    d[n - 1] = (points[n - 1] - points[n - 2]) / 3.0;

    // Forward sweep (Thomas algorithm): d[i] = a[i] + b[i] * d[i + 1].
    b[1] = -0.25;
    a[1] = (points[2] - points[0] - d[0]) / 4.0;
    for i in 2..n - 1 {
        b[i] = -1.0 / (4.0 + b[i - 1]);
        a[i] = -(points[i + 1] - points[i - 1] - a[i - 1]) * b[i];
    }
    // Back substitution for every inner point, including the one next to the end.
    for i in (1..n - 1).rev() {
        d[i] = a[i] + d[i + 1] * b[i];
    }
    d
}

/// End points of the two strokes forming an arrowhead whose tip is at `tip`,
/// pointing away from `tail`. `None` when the arrow has no direction.
pub fn arrow_head(tail: Point, tip: Point, length: f64, half_angle: f64) -> Option<(Point, Point)> {
    let dx = tip.0 - tail.0;
    let dy = tip.1 - tail.1;
    if dx == 0.0 && dy == 0.0 {
        return None;
    }
    let angle = dy.atan2(dx);
    let wing = |a: f64| Point(tip.0 - length * a.cos(), tip.1 - length * a.sin());
    Some((wing(angle - half_angle), wing(angle + half_angle)))
}

/// Distance from `p` to the segment from `a` to `b`.
pub fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let len_sq = ab.0 * ab.0 + ab.1 * ab.1;
    let t = if len_sq == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * ab.0 + (p.1 - a.1) * ab.1) / len_sq).clamp(0.0, 1.0)
    };
    distance_sq(p, a + ab * t).sqrt()
}

/// Distance from `p` to the line through `points` (to the point itself for one point,
/// infinite for none).
pub fn distance_to_polyline(p: Point, points: &[Point]) -> f64 {
    match points {
        [] => f64::INFINITY,
        [only] => distance_sq(p, *only).sqrt(),
        _ => points
            .windows(2)
            .map(|w| distance_to_segment(p, w[0], w[1]))
            .fold(f64::INFINITY, f64::min),
    }
}

/// Top-left and bottom-right corners of the box around `points`, grown by `margin` on
/// every side. `None` for no points.
pub fn bounding_box(points: &[Point], margin: f64) -> Option<(Point, Point)> {
    let (first, rest) = points.split_first()?;
    let (min, max) = rest.iter().fold((*first, *first), |(min, max), p| {
        (
            Point(min.0.min(p.0), min.1.min(p.1)),
            Point(max.0.max(p.0), max.1.max(p.1)),
        )
    });
    Some((min - Point(margin, margin), max + Point(margin, margin)))
}
