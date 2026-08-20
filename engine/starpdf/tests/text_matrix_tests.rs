use starpdf::text::Matrix2D;

#[test]
fn test_matrix_identity_and_translation() {
    let identity = Matrix2D::identity();
    let (x, y) = identity.transform_point(10.0, 20.0);
    assert_eq!((x, y), (10.0, 20.0));

    let trans = Matrix2D::translation(50.0, 100.0);
    let (tx, ty) = trans.transform_point(10.0, 20.0);
    assert_eq!((tx, ty), (60.0, 120.0));
}

#[test]
fn test_matrix_scaling_and_point_transformation() {
    let scale = Matrix2D::scaling(2.0, 3.0);
    let (sx, sy) = scale.transform_point(10.0, 20.0);
    assert_eq!((sx, sy), (20.0, 60.0));
    assert_eq!(scale.scale_x(), 2.0);
    assert_eq!(scale.scale_y(), 3.0);
}

#[test]
fn test_matrix_rotation_angles() {
    // 0 deg
    let m0 = Matrix2D::rotation(0.0);
    assert!((m0.rotation_degrees() - 0.0).abs() < 1e-6);

    // 90 deg
    let m90 = Matrix2D::rotation(std::f64::consts::FRAC_PI_2);
    assert!((m90.rotation_degrees() - 90.0).abs() < 1e-6);

    // 180 deg
    let m180 = Matrix2D::rotation(std::f64::consts::PI);
    assert!((m180.rotation_degrees() - 180.0).abs() < 1e-6);

    // 270 deg
    let m270 = Matrix2D::rotation(3.0 * std::f64::consts::FRAC_PI_2);
    assert!((m270.rotation_degrees() - 270.0).abs() < 1e-6);
}

#[test]
fn test_matrix_multiplication_composite() {
    // Translate by (10, 20) then scale by (2, 2)
    let t = Matrix2D::translation(10.0, 20.0);
    let s = Matrix2D::scaling(2.0, 2.0);
    let combined = t.multiply(&s);

    let (x, y) = combined.transform_point(5.0, 5.0);
    assert_eq!((x, y), (30.0, 50.0));
}
