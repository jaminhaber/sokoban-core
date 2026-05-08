use std::collections::HashSet;

use rustc_hash::FxHashSet;
use sokoban_core::path_finding::*;
use sokoban_core::IVector2;

mod utils;
use utils::*;

#[test]
fn player_move_path_on_microban() {
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
    assert_eq!(
        pushable_boxes(&map),
        FxHashSet::from_iter([IVector2::new(6, 3)])
    );
}

#[test]
fn reachable_area_in_open_room() {
    // 3×3 floor enclosed in walls — every interior cell is reachable.
    let blocked = |p: IVector2| p.x < 1 || p.x > 3 || p.y < 1 || p.y > 3;
    let area = reachable_area(IVector2::new(2, 2), |p| !blocked(p));
    assert_eq!(area.len(), 9);
}

#[test]
fn reachable_area_respects_blockers() {
    // A vertical wall splits the room in half. Only the player's side is reachable.
    let area = reachable_area(IVector2::new(0, 0), |p| {
        // Floor is everywhere except the column x = 2.
        p.x != 2 && (0..5).contains(&p.x) && (0..3).contains(&p.y)
    });
    // Cells with x in {0, 1} for y in {0, 1, 2}: 6 reachable cells.
    assert_eq!(area.len(), 6);
}

#[test]
fn reachable_area_with_distances_returns_bfs_distances() {
    // Open 4×4 floor with no obstacles.
    let dist = reachable_area_with_distances(IVector2::new(0, 0), |p| {
        (0..4).contains(&p.x) && (0..4).contains(&p.y)
    });
    assert_eq!(dist[&IVector2::new(0, 0)], 0);
    assert_eq!(dist[&IVector2::new(1, 0)], 1);
    assert_eq!(dist[&IVector2::new(0, 1)], 1);
    // Manhattan distance from (0,0) to (3,3) is 6.
    assert_eq!(dist[&IVector2::new(3, 3)], 6);
    assert_eq!(dist.len(), 16);
}

#[test]
fn normalized_area_picks_lex_min_by_y_then_x() {
    // Picks the lexicographically smallest (y, x) — the bottom-left under
    // this crate's Y convention. Of these three cells, (1, 0) wins on the
    // smallest y, breaking the tie with (3, 0) on smaller x.
    let area: FxHashSet<IVector2> = [
        IVector2::new(2, 5),
        IVector2::new(1, 0),
        IVector2::new(3, 0),
    ]
    .into_iter()
    .collect();
    assert_eq!(normalized_area(&area), Some(IVector2::new(1, 0)));
}

#[test]
fn normalized_area_of_empty_set_is_none() {
    let empty: FxHashSet<IVector2> = FxHashSet::default();
    assert_eq!(normalized_area(&empty), None);
}

/// `find_path` is A* over the grid implied by the `can_move` predicate. The
/// predicate must bound *both* axes — otherwise the search wanders off into
/// the infinite 2D plane. These tests use closed (0..N) ranges on x AND y.
#[test]
fn find_path_returns_none_when_unreachable() {
    // 5×3 grid with column x = 2 fully blocked → no path from left to right.
    let path = find_path(IVector2::new(0, 0), IVector2::new(4, 0), |p| {
        p.x != 2 && (0..5).contains(&p.x) && (0..3).contains(&p.y)
    });
    assert!(path.is_none());
}

#[test]
fn find_path_includes_endpoints() {
    let path = find_path(IVector2::new(0, 0), IVector2::new(2, 0), |p| {
        (0..5).contains(&p.x) && (0..1).contains(&p.y)
    })
    .unwrap();
    assert_eq!(*path.first().unwrap(), IVector2::new(0, 0));
    assert_eq!(*path.last().unwrap(), IVector2::new(2, 0));
}

#[test]
fn find_path_to_self_is_a_single_node() {
    let path = find_path(IVector2::new(0, 0), IVector2::new(0, 0), |p| {
        (0..3).contains(&p.x) && (0..3).contains(&p.y)
    })
    .unwrap();
    assert_eq!(path, vec![IVector2::new(0, 0)]);
}
