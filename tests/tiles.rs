//! Tests for the `Tiles` bitflag and its `Display` impl, which is used to
//! render maps back to the XSB-like character set.

use sokoban_core::tiles::Tiles;

fn render(tiles: Tiles) -> String {
    format!("{}", tiles)
}

#[test]
fn empty_renders_as_dash() {
    assert_eq!(render(Tiles::empty()), "-");
}

#[test]
fn lone_floor_renders_as_underscore() {
    assert_eq!(render(Tiles::Floor), "_");
}

#[test]
fn wall_renders_as_hash() {
    assert_eq!(render(Tiles::Wall), "#");
}

#[test]
fn box_on_floor_renders_as_dollar() {
    assert_eq!(render(Tiles::Floor | Tiles::Box), "$");
}

#[test]
fn goal_on_floor_renders_as_period() {
    assert_eq!(render(Tiles::Floor | Tiles::Goal), ".");
}

#[test]
fn player_on_floor_renders_as_at_sign() {
    assert_eq!(render(Tiles::Floor | Tiles::Player), "@");
}

#[test]
fn box_on_goal_renders_as_star() {
    assert_eq!(render(Tiles::Floor | Tiles::Box | Tiles::Goal), "*");
}

#[test]
fn player_on_goal_renders_as_plus() {
    assert_eq!(render(Tiles::Floor | Tiles::Player | Tiles::Goal), "+");
}

#[test]
fn unrecognized_combination_renders_as_question_mark() {
    // Box and player on the same cell isn't a valid Sokoban configuration —
    // the renderer signals it with `?` rather than crashing.
    let weird = Tiles::Floor | Tiles::Box | Tiles::Player;
    assert_eq!(render(weird), "?");
}

#[test]
fn flag_arithmetic_works_as_expected() {
    let mut t = Tiles::Floor | Tiles::Box;
    assert!(t.intersects(Tiles::Floor));
    assert!(t.intersects(Tiles::Box));
    assert!(!t.intersects(Tiles::Goal));

    t.remove(Tiles::Box);
    assert!(t.intersects(Tiles::Floor));
    assert!(!t.intersects(Tiles::Box));

    t.insert(Tiles::Goal);
    assert!(t.contains(Tiles::Floor | Tiles::Goal));
}
