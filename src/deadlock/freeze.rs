//! Freeze deadlock detection.
//!
//! A box is *frozen* if both its movement axes are blocked — by walls or by
//! other frozen boxes (recursively). A configuration is a freeze deadlock if
//! any frozen box is off-goal: that box is permanently stuck and the puzzle
//! can no longer be solved.

use std::collections::HashSet;

use crate::{box_set::BoxSet, direction::Direction, map::Map, math::IVector2, tiles::Tiles};

/// Returns `true` iff pushing a box to `box_position` creates a freeze
/// deadlock.
///
/// A freeze deadlock occurs when a connected group of mutually immovable boxes
/// (the *frozen component*) contains at least one box that is not on a goal:
/// that box is permanently stuck off-goal, so the puzzle can no longer be
/// solved.
///
/// Importantly, it is not sufficient to check that the *pushed* box is on a
/// goal. Pushing onto a goal can freeze a neighbor that is off-goal, which is
/// still a deadlock. This helper walks the whole frozen component and reports
/// a deadlock if any member is off-goal.
pub fn introduces_freeze_deadlock(
    map: &Map,
    box_position: IVector2,
    box_positions: &BoxSet,
) -> bool {
    let mut frozen = HashSet::new();
    if !is_freeze_deadlock(map, box_position, box_positions, &mut frozen) {
        return false;
    }
    frozen.iter().any(|p| !map[*p].intersects(Tiles::Goal))
}

/// Checks if the given box position is a freeze deadlock.
///
/// On return, `visited` contains the connected component of mutually-frozen
/// boxes — useful to the caller for further analysis (e.g. checking goal
/// status of every frozen member, as [`introduces_freeze_deadlock`] does).
pub fn is_freeze_deadlock(
    map: &Map,
    box_position: IVector2,
    box_positions: &BoxSet,
    visited: &mut HashSet<IVector2>,
) -> bool {
    debug_assert!(box_positions.contains(&box_position));

    if !visited.insert(box_position) {
        return true;
    }

    for direction in [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ]
    .chunks(2)
    {
        let neighbors = [
            box_position + &direction[0].into(),
            box_position + &direction[1].into(),
        ];

        // Axis blocked by walls?
        if map[neighbors[0]].intersects(Tiles::Wall) || map[neighbors[1]].intersects(Tiles::Wall) {
            continue;
        }

        // Axis blocked by frozen-box neighbors?
        if (box_positions.contains(&neighbors[0])
            && is_freeze_deadlock(map, neighbors[0], box_positions, visited))
            || (box_positions.contains(&neighbors[1])
                && is_freeze_deadlock(map, neighbors[1], box_positions, visited))
        {
            continue;
        }

        return false;
    }
    true
}

/// Calculates the positions of useless boxes — boxes that are already frozen
/// in the initial configuration. Used during map normalization to convert
/// such cells into walls (since they can never move).
pub fn calculate_useless_boxes(map: &Map) -> HashSet<IVector2> {
    map.box_positions()
        .iter()
        .filter(|&position| {
            is_freeze_deadlock(map, position, map.box_positions(), &mut HashSet::new())
        })
        .collect()
}
