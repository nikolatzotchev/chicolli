use chicolli::geometry::{
    arrow_head, bounding_box, distance_sq, distance_to_polyline, distance_to_segment, snap_angle,
    snap_square, spline_controls, Point,
};

const EPSILON: f64 = 1e-9;

fn assert_point_close(actual: Point, expected: Point) {
    assert!((actual.0 - expected.0).abs() < EPSILON);
    assert!((actual.1 - expected.1).abs() < EPSILON);
}

#[test]
fn point_arithmetic_operators_work() {
    let left = Point(5.0, -2.0);
    let right = Point(2.0, 6.0);

    assert_point_close(left + right, Point(7.0, 4.0));
    assert_point_close(left - right, Point(3.0, -8.0));
    assert_point_close(left * 2.0, Point(10.0, -4.0));
    assert_point_close(left / 2.0, Point(2.5, -1.0));
    assert_point_close(-left, Point(-5.0, 2.0));
}

#[test]
fn snap_angle_snaps_to_nearest_45_degree_angle() {
    let start = Point(0.0, 0.0);
    let end = Point(2.0, 1.0);

    let snapped = snap_angle(start, end);

    let expected_len = (2.0_f64.powi(2) + 1.0_f64.powi(2)).sqrt();
    let expected = Point(
        expected_len * std::f64::consts::FRAC_1_SQRT_2,
        expected_len * std::f64::consts::FRAC_1_SQRT_2,
    );
    assert_point_close(snapped, expected);
}

#[test]
fn snap_square_makes_square_in_negative_direction() {
    let start = Point(5.0, 5.0);
    let end = Point(2.0, 3.0);

    let snapped = snap_square(start, end);

    assert_point_close(snapped, Point(2.0, 2.0));
}

#[test]
fn spline_controls_solve_every_inner_equation() {
    let points = [
        Point(0.0, 0.0),
        Point(10.0, 5.0),
        Point(20.0, -3.0),
        Point(30.0, 8.0),
        Point(40.0, 0.0),
        Point(55.0, 12.0),
    ];
    let d = spline_controls(&points);
    assert_eq!(d.len(), points.len());
    // Every inner point, including the one next to the end, must satisfy
    // d[i - 1] + 4 d[i] + d[i + 1] = p[i + 1] - p[i - 1].
    for i in 1..points.len() - 1 {
        let lhs = d[i - 1] + d[i] * 4.0 + d[i + 1];
        let rhs = points[i + 1] - points[i - 1];
        assert!(distance_sq(lhs, rhs) < 1e-12, "row {i}: {lhs:?} != {rhs:?}");
    }
}

#[test]
fn spline_through_collinear_points_is_straight() {
    let points: Vec<Point> = (0..5).map(|i| Point(f64::from(i) * 10.0, 0.0)).collect();
    for c in spline_controls(&points) {
        assert!((c.0 - 10.0 / 3.0).abs() < EPSILON);
        assert!(c.1.abs() < EPSILON);
    }
}

#[test]
fn spline_controls_need_three_points() {
    assert!(spline_controls(&[Point(0.0, 0.0), Point(1.0, 1.0)]).is_empty());
    assert_eq!(
        spline_controls(&[Point(0.0, 0.0), Point(1.0, 1.0), Point(2.0, 0.0)]).len(),
        3
    );
}

#[test]
fn arrow_head_wings_trail_behind_the_tip() {
    let (w1, w2) = arrow_head(
        Point(0.0, 0.0),
        Point(100.0, 0.0),
        20.0,
        std::f64::consts::FRAC_PI_4,
    )
    .expect("arrow has a direction");
    let back = 20.0 * std::f64::consts::FRAC_1_SQRT_2;
    assert_point_close(w1, Point(100.0 - back, back));
    assert_point_close(w2, Point(100.0 - back, -back));
}

#[test]
fn arrow_head_without_direction_is_none() {
    assert!(arrow_head(Point(3.0, 3.0), Point(3.0, 3.0), 20.0, 0.5).is_none());
}

#[test]
fn distance_to_segment_clamps_to_the_ends() {
    let (a, b) = (Point(0.0, 0.0), Point(10.0, 0.0));
    assert!((distance_to_segment(Point(5.0, 3.0), a, b) - 3.0).abs() < EPSILON);
    assert!((distance_to_segment(Point(-4.0, 3.0), a, b) - 5.0).abs() < EPSILON);
    assert!((distance_to_segment(Point(13.0, 4.0), a, b) - 5.0).abs() < EPSILON);
    // A zero-length segment is a point.
    assert!((distance_to_segment(Point(3.0, 4.0), a, a) - 5.0).abs() < EPSILON);
}

#[test]
fn distance_to_polyline_takes_the_nearest_segment() {
    let line = [Point(0.0, 0.0), Point(10.0, 0.0), Point(10.0, 10.0)];
    assert!((distance_to_polyline(Point(12.0, 5.0), &line) - 2.0).abs() < EPSILON);
    assert!((distance_to_polyline(Point(5.0, -1.0), &line) - 1.0).abs() < EPSILON);
    assert!((distance_to_polyline(Point(3.0, 4.0), &line[..1]) - 5.0).abs() < EPSILON);
    assert!(distance_to_polyline(Point(0.0, 0.0), &[]).is_infinite());
}

#[test]
fn bounding_box_spans_every_point_plus_margin() {
    let (min, max) =
        bounding_box(&[Point(3.0, 8.0), Point(-1.0, 2.0), Point(5.0, 4.0)], 1.0).unwrap();
    assert_point_close(min, Point(-2.0, 1.0));
    assert_point_close(max, Point(6.0, 9.0));
    assert!(bounding_box(&[], 1.0).is_none());
}
