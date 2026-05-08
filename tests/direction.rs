use sokoban_core::{direction::*, IVector2};

#[test]
fn rotate() {
    use Direction::*;
    assert_eq!(Up.rotate(), Right);
    assert_eq!(Right.rotate(), Down);
    assert_eq!(Down.rotate(), Left);
    assert_eq!(Left.rotate(), Up);
}

#[test]
fn flip() {
    use Direction::*;
    assert_eq!(Up.flip(), Down);
    assert_eq!(Down.flip(), Up);
    assert_eq!(Right.flip(), Left);
    assert_eq!(Left.flip(), Right);
}

#[test]
fn neg_is_flip() {
    use Direction::*;
    assert_eq!(-Up, Down);
    assert_eq!(-Down, Up);
    assert_eq!(-Right, Left);
    assert_eq!(-Left, Right);
}

#[test]
fn iter_visits_all_four_directions() {
    use Direction::*;
    let collected: Vec<Direction> = Direction::iter().collect();
    assert_eq!(collected, vec![Up, Down, Left, Right]);
}

#[test]
fn perpendiculars() {
    use Direction::*;
    assert_eq!(Up.perpendiculars(), (Left, Right));
    assert_eq!(Down.perpendiculars(), (Left, Right));
    assert_eq!(Left.perpendiculars(), (Up, Down));
    assert_eq!(Right.perpendiculars(), (Up, Down));
}

#[test]
fn into_vector_uses_screen_convention() {
    use Direction::*;
    assert_eq!(IVector2::from(Up), IVector2::new(0, 1));
    assert_eq!(IVector2::from(Down), IVector2::new(0, -1));
    assert_eq!(IVector2::from(Left), IVector2::new(-1, 0));
    assert_eq!(IVector2::from(Right), IVector2::new(1, 0));
}

#[test]
fn try_from_axis_aligned_unit_vectors() {
    use Direction::*;
    assert_eq!(Direction::try_from(IVector2::new(0, 1)), Ok(Up));
    assert_eq!(Direction::try_from(IVector2::new(0, -1)), Ok(Down));
    assert_eq!(Direction::try_from(IVector2::new(-1, 0)), Ok(Left));
    assert_eq!(Direction::try_from(IVector2::new(1, 0)), Ok(Right));
}

#[test]
fn try_from_rejects_non_unit_or_diagonal() {
    assert!(Direction::try_from(IVector2::new(0, 0)).is_err());
    assert!(Direction::try_from(IVector2::new(2, 0)).is_err());
    assert!(Direction::try_from(IVector2::new(1, 1)).is_err());
    assert!(Direction::try_from(IVector2::new(-1, -1)).is_err());
}
