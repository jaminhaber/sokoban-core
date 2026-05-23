//! Tests for the integer 2D vector type used for grid positions.

use sokoban_core::IVector2;

#[test]
fn constructors() {
    assert_eq!(IVector2::new(3, -7), IVector2 { x: 3, y: -7 });
    assert_eq!(IVector2::zeros(), IVector2 { x: 0, y: 0 });
    assert_eq!(IVector2::x_axis(), IVector2 { x: 1, y: 0 });
    assert_eq!(IVector2::y_axis(), IVector2 { x: 0, y: 1 });
    assert_eq!(IVector2::unit_x(), IVector2::x_axis());
    assert_eq!(IVector2::unit_y(), IVector2::y_axis());
    assert_eq!(IVector2::neg_x_axis(), IVector2::new(-1, 0));
    assert_eq!(IVector2::neg_y_axis(), IVector2::new(0, -1));
}

#[test]
fn abs_and_signum() {
    assert_eq!(IVector2::new(-3, 5).abs(), IVector2::new(3, 5));
    assert_eq!(IVector2::new(0, -7).abs(), IVector2::new(0, 7));
    assert_eq!(IVector2::new(-3, 5).signum(), IVector2::new(-1, 1));
    assert_eq!(IVector2::new(0, -7).signum(), IVector2::new(0, -1));
    assert_eq!(IVector2::zeros().signum(), IVector2::zeros());
}

#[test]
fn manhattan_distance_is_symmetric_and_zero_at_self() {
    let a = IVector2::new(1, 2);
    let b = IVector2::new(4, -3);
    assert_eq!(a.manhattan_distance(b), 8);
    assert_eq!(b.manhattan_distance(a), 8);
    assert_eq!(a.manhattan_distance(a), 0);
}

#[test]
fn rotation_90_cw_and_ccw_are_inverses() {
    let v = IVector2::new(3, 7);
    assert_eq!(v.rotate_90_cw().rotate_90_ccw(), v);
    // Four CW rotations is the identity.
    let mut w = v;
    for _ in 0..4 {
        w = w.rotate_90_cw();
    }
    assert_eq!(w, v);
}

#[test]
fn rotation_directions_match_screen_convention() {
    // Up (0,1) rotates 90° CW to Right (1,0).
    assert_eq!(IVector2::y_axis().rotate_90_cw(), IVector2::x_axis());
    // Up rotates 90° CCW to Left (-1,0).
    assert_eq!(IVector2::y_axis().rotate_90_ccw(), -IVector2::x_axis());
}

#[test]
fn yx_swaps_components() {
    assert_eq!(IVector2::new(3, 7).yx(), IVector2::new(7, 3));
}

#[test]
fn zip_map_combines_componentwise() {
    let a = IVector2::new(3, 9);
    let b = IVector2::new(5, 4);
    assert_eq!(a.zip_map(&b, std::cmp::min), IVector2::new(3, 4));
    assert_eq!(a.zip_map(&b, std::cmp::max), IVector2::new(5, 9));
    assert_eq!(a.zip_map(&b, |x, y| x + y), IVector2::new(8, 13));
}

#[test]
fn sum_returns_x_plus_y() {
    assert_eq!(IVector2::new(3, 7).sum(), 10);
    assert_eq!(IVector2::new(-3, 7).sum(), 4);
    assert_eq!(IVector2::zeros().sum(), 0);
}

#[test]
fn iter_yields_x_then_y() {
    let v = IVector2::new(3, 7);
    let collected: Vec<i32> = v.iter().copied().collect();
    assert_eq!(collected, vec![3, 7]);
}

#[test]
fn iter_mut_allows_mutation() {
    let mut v = IVector2::new(3, 7);
    for component in v.iter_mut() {
        *component *= 2;
    }
    assert_eq!(v, IVector2::new(6, 14));
}

#[test]
#[allow(clippy::op_ref)]
fn add_sub_neg_arithmetic() {
    let a = IVector2::new(3, 7);
    let b = IVector2::new(1, 4);

    assert_eq!(a + b, IVector2::new(4, 11));
    assert_eq!(a - b, IVector2::new(2, 3));
    assert_eq!(-a, IVector2::new(-3, -7));
    // Reference variants should match by-value variants.
    assert_eq!(a + &b, a + b);
    assert_eq!(&a + b, a + b);
    assert_eq!(&a + &b, a + b);
    assert_eq!(a - &b, a - b);
    assert_eq!(&a - &b, a - b);
}

#[test]
fn add_assign_and_sub_assign() {
    let mut a = IVector2::new(3, 7);
    a += IVector2::new(1, 2);
    assert_eq!(a, IVector2::new(4, 9));

    a += &IVector2::new(1, 1);
    assert_eq!(a, IVector2::new(5, 10));

    a -= IVector2::new(2, 3);
    assert_eq!(a, IVector2::new(3, 7));
}

#[test]
fn index_and_index_mut() {
    let v = IVector2::new(3, 7);
    assert_eq!(v[0], 3);
    assert_eq!(v[1], 7);

    let mut w = IVector2::new(3, 7);
    w[0] = 100;
    w[1] = -100;
    assert_eq!(w, IVector2::new(100, -100));
}

#[test]
#[should_panic]
fn index_out_of_bounds_panics() {
    let v = IVector2::new(3, 7);
    let _ = v[2];
}

#[test]
fn display_format() {
    assert_eq!(format!("{}", IVector2::new(3, 7)), "(3, 7)");
    assert_eq!(format!("{}", IVector2::new(-1, 0)), "(-1, 0)");
}

#[test]
fn ord_is_lexicographic_x_then_y() {
    // Derived `Ord` compares fields in declaration order: x first, then y.
    assert!(IVector2::new(1, 0) < IVector2::new(2, 0));
    assert!(IVector2::new(1, 0) < IVector2::new(1, 1));
    assert!(IVector2::new(2, 0) > IVector2::new(1, 100));
}
