use std::collections::HashSet;

use sokoban_core::path_finding::*;
use sokoban_core::IVector2;

mod utils;
use utils::*;

#[test]
fn find_path() {
    let map: sokoban_core::Map = load_level_from_file("assets/Microban II_135.xsb", 132).into();
    let path = player_move_path(&map, IVector2::new(25, 5)).unwrap();
    assert_eq!(path.len(), 41);
}

#[test]
fn test_box_move_waypoints() {
    let map = load_level_from_file("assets/Microban_155.xsb", 3).into();
    assert_eq!(box_move_waypoints(&map, IVector2::new(6, 2)).len(), 0);
    let waypoints = box_move_waypoints(&map, IVector2::new(6, 3));
    let positions: HashSet<_> = waypoints.iter().map(|((pos, _), _)| pos).collect();
    assert_eq!(positions.len(), 15);

    let map: sokoban_core::Map = load_level_from_file("assets/Microban II_135.xsb", 132).into();
    let waypoints = box_move_waypoints(&map, IVector2::new(8, 19));
    let positions: HashSet<_> = waypoints.iter().map(|((pos, _), _)| pos).collect();
    let box_path = construct_box_path(IVector2::new(8, 19), IVector2::new(9, 18), &waypoints);
    let player_path = construct_player_path(&map, IVector2::new(7, 20), &box_path);
    assert_eq!(positions.len(), 4 * 35);
    assert_eq!(box_path.len() - 1, 110);
    assert_eq!(player_path.len() - 1, 487);

    let map: sokoban_core::Map = load_level_from_file("assets/Microban II_135.xsb", 133).into();
    let waypoints = box_move_waypoints(&map, IVector2::new(18, 15));
    let positions: HashSet<_> = waypoints.iter().map(|((pos, _), _)| pos).collect();
    let box_path = construct_box_path(IVector2::new(18, 15), IVector2::new(17, 15), &waypoints);
    let player_path = construct_player_path(&map, IVector2::new(16, 15), &box_path);
    assert_eq!(positions.len(), 4 * 6);
    assert_eq!(box_path.len() - 1, 11);
    assert_eq!(player_path.len() - 1, 618);

    let map: sokoban_core::Map = load_level_from_file("assets/Microban II_135.xsb", 134).into();
    let waypoints = box_move_waypoints(&map, IVector2::new(16, 2));
    let box_path = construct_box_path(IVector2::new(16, 2), IVector2::new(20, 2), &waypoints);
    let player_path = construct_player_path(&map, IVector2::new(18, 18), &box_path);
    assert_eq!(box_path.len() - 1, 124);
    assert_eq!(player_path.len() - 1, 5037);

    // FIXME:
    // let map = load_level_from_file("assets/Microban II_135.xsb", 135).into();
    // let waypoints = box_move_waypoints(&map, Vec2::new(21, 36));
    // let box_path = construct_box_path(Vec2::new(21, 36), Vec2::new(21,
    // 37), &waypoints); assert_eq!(box_path.len() - 1, 591);
    // let player_path = construct_player_path(&map, Vec2::new(21, 38),
    // &box_path); assert_eq!(player_path.len() - 1, 1108);
}

#[test]
fn test_pushable_boxes() {
    let map = load_level_from_file("assets/Microban_155.xsb", 3).into();
    assert_eq!(pushable_boxes(&map), HashSet::from([IVector2::new(6, 3)]));
}
