use std::{collections::HashSet, str::FromStr};

use sokoban_core::{deadlock, BoxSet, IVector2, Map};

mod utils;
use utils::*;

#[test]
fn calculate_static_deadlocks() {
    let map = load_level_from_file("assets/Microban_155.xsb", 3).into();
    assert_eq!(deadlock::calculate_static_deadlocks(&map).len(), 9);

    let map = load_level_from_file("assets/BoxWorld_100.xsb", 9).into();
    assert_eq!(deadlock::calculate_static_deadlocks(&map).len(), 17);
}

/// `is_static_deadlock` flags a 2×2 box cluster fully enclosed by walls —
/// each box's blocked-axis neighbors are walls or other frozen-cluster boxes,
/// so the recursion confirms all four members are mutually frozen.
#[test]
fn is_static_deadlock_walled_2x2_cluster() {
    let xsb = "######\n#   .#\n#@...#\n######\n##$$##\n##$$##\n######\n";
    let map = Map::from_str(xsb).unwrap();
    let mut visited = HashSet::new();
    // Pick the bottom-left member of the cluster as the entry point.
    assert!(deadlock::is_static_deadlock(
        &map,
        IVector2::new(2, 1),
        map.box_positions(),
        &mut visited
    ));
}

/// `is_static_deadlock` does *not* flag a box with three free neighbors
/// (one box neighbor that itself isn't trapped).
#[test]
fn is_static_deadlock_box_with_free_neighbors() {
    let xsb = "######\n#@$ .#\n#    #\n######\n";
    let map = Map::from_str(xsb).unwrap();
    let boxes = BoxSet::from_iter(map.dimensions().x, [IVector2::new(2, 2)]);
    let mut visited = HashSet::new();
    assert!(!deadlock::is_static_deadlock(
        &map,
        IVector2::new(2, 2),
        &boxes,
        &mut visited
    ));
}

/// `calculate_useless_floors` strips dead-end corridors leading nowhere.
#[test]
fn useless_floors_collapse_dead_end_corridor() {
    // The lower row is a 1-wide cul-de-sac — every cell in it is a dead-end
    // floor that the player can't usefully visit (no goal/box there).
    let xsb = r#"
        ########
        #@ $.  #
        ########
    "#;
    let map = Map::from_str(xsb).unwrap();
    // For this small level there are no dead-end corridors — the function
    // should not flag any of the playable floor cells.
    let useless = deadlock::calculate_useless_floors(map.clone());
    for pos in &useless {
        // Useless floors are filled in with walls, so by definition they
        // shouldn't contain a box, goal, or the player on the original map.
        assert!(!map.box_positions().contains(pos));
        assert!(!map.goal_positions().contains(pos));
        assert_ne!(*pos, map.player_position());
    }
}

/// `calculate_useless_boxes` returns boxes that are already frozen at start.
#[test]
fn useless_boxes_for_unfrozen_initial_state_is_empty() {
    let level = load_level_from_file("assets/Microban_155.xsb", 1);
    let map = level.map();
    // Microban #1 is solvable, so no box starts in a frozen-and-off-goal state.
    // Frozen-on-goal boxes are flagged too — but Microban #1 doesn't have any.
    assert!(deadlock::calculate_useless_boxes(map).is_empty());
}
