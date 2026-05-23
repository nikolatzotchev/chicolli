use gtk4_drawing_tool::geometry::{snap_angle, snap_square, Point};

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
