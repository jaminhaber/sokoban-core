//! Utilities for deadlocks detection.

use std::collections::{HashSet, VecDeque};

use crate::{box_set::BoxSet, direction::Direction, map::Map, math::IVector2, tiles::Tiles};

/// Checks if the given box position is a static deadlock.
///
/// Consider using [`calculate_static_deadlocks`] if you need to efficiently
/// compute multiple static deadlock positions.
pub fn is_static_deadlock(
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
        Direction::Right,
        Direction::Down,
        Direction::Left,
    ]
    .windows(3)
    {
        let neighbors = [
            box_position + &direction[0].into(),
            box_position + &direction[1].into(),
            box_position + &direction[2].into(),
        ];
        for neighbor in &neighbors {
            if map[*neighbor].intersects(Tiles::Wall) {
                continue;
            }
            if box_positions.contains(neighbor)
                && is_static_deadlock(map, *neighbor, box_positions, visited)
            {
                continue;
            }
            return false;
        }
    }
    true
}

/// Returns `true` iff pushing a box to `box_position` creates a freeze deadlock.
///
/// A freeze deadlock occurs when a connected group of mutually immovable boxes
/// (the *frozen component*) contains at least one box that is not on a goal:
/// that box is permanently stuck off-goal, so the puzzle can no longer be solved.
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

        // Check if any immovable walls on the axis.
        if map[neighbors[0]].intersects(Tiles::Wall) || map[neighbors[1]].intersects(Tiles::Wall) {
            continue;
        }

        // Check if any immovable boxes on the axis.
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

/// Calculates static deadlock positions independent of the player's position.
///
/// This function returns an **incomplete** set of dead positions independent
/// of the player's position. Any box pushed to a point in the set will cause a
/// deadlock, regardless of the player's position.
pub fn calculate_static_deadlocks(map: &Map) -> HashSet<IVector2> {
    let mut dead_positions = HashSet::new();
    for x in 1..map.dimensions().x - 1 {
        for y in 1..map.dimensions().y - 1 {
            let position = IVector2::new(x, y);
            // Check if current position may be a new corner
            if !map[position].intersects(Tiles::Floor) || map[position].intersects(Tiles::Goal) {
                continue;
            }
            for directions in [
                Direction::Up,
                Direction::Right,
                Direction::Down,
                Direction::Left,
                Direction::Up,
            ]
            .windows(2)
            {
                let neighbor = [
                    position + &directions[0].into(),
                    position + &directions[1].into(),
                ];

                // Check whether the current position is a corner
                if !(map[neighbor[0]].intersects(Tiles::Wall)
                    && map[neighbor[1]].intersects(Tiles::Wall))
                {
                    continue;
                }
                dead_positions.insert(position);

                // Detects grooves based on current position
                let mut potential_dead_positions = HashSet::new();
                let mut next_position = position - &(directions[0]).into();
                while map[next_position + &directions[1].into()].intersects(Tiles::Wall) {
                    if map[next_position].intersects(Tiles::Goal) {
                        break;
                    }
                    if map[next_position].intersects(Tiles::Wall) {
                        dead_positions.extend(potential_dead_positions);
                        break;
                    }
                    potential_dead_positions.insert(next_position);
                    next_position -= (directions[0]).into();
                }
            }
        }
    }
    dead_positions
}

/// Calculate the positions of the useless floors.
pub fn calculate_useless_floors(mut map: Map) -> HashSet<IVector2> {
    let mut useless_floors = HashSet::new();

    // Add all floors to `unchecked_floors`
    let mut unchecked_floors = VecDeque::new();
    for y in 1..map.dimensions().y - 1 {
        for x in 1..map.dimensions().x - 1 {
            let position = IVector2::new(x, y);
            if map[position] == Tiles::Floor {
                unchecked_floors.push_back(position);
            }
        }
    }

    while let Some(position) = unchecked_floors.pop_front() {
        // Check if the current floor is in a dead end and store the exit position in
        // `neighbor_floor`
        let mut neighbor_floor = None;
        for direction in Direction::iter() {
            let neighbor = position + &direction.into();
            if !map[neighbor].intersects(Tiles::Wall) {
                if neighbor_floor.is_some() {
                    neighbor_floor = None;
                    break;
                }
                neighbor_floor = Some(neighbor);
            }
        }
        // If the current floor is in a dead end
        if let Some(free_neighbor) = neighbor_floor {
            useless_floors.insert(position);
            map[position].remove(Tiles::Floor);
            map[position].insert(Tiles::Wall);
            // As the only floor affected by terrain changes, `free_neighbor` may become a
            // new unused floor and needs to be rechecked.
            if map[free_neighbor] == Tiles::Floor && !map[free_neighbor].intersects(Tiles::Wall) {
                unchecked_floors.push_back(free_neighbor);
            }
        }
    }

    useless_floors
}

/// Calculate the positions of the useless boxes.
pub fn calculate_useless_boxes(map: &Map) -> HashSet<IVector2> {
    map.box_positions()
        .iter()
        .filter(|&position| {
            is_freeze_deadlock(map, position, map.box_positions(), &mut HashSet::new())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    /// Vertical corridor with two boxes and two goals. Pushing the lower box up
    /// onto the lower goal puts the upper box (off-goal) into a position where
    /// its only escape is downward — but downward is now blocked by the just-
    /// pushed box. Both boxes are frozen; the upper one is off-goal, so this
    /// is a deadlock.
    ///
    /// Layout (XSB top → bottom):
    /// ```text
    /// #####
    /// ##.##   upper goal  (code-y = 5)
    /// ##$##   upper box   (code-y = 4)  <- becomes frozen off-goal
    /// ##.##   lower goal  (code-y = 3)  <- pushed-onto goal
    /// ##$##   lower box   (code-y = 2)
    /// ##@##   player      (code-y = 1)
    /// #####
    /// ```
    const TWO_BOX_CORRIDOR: &str = "#####\n##.##\n##$##\n##.##\n##$##\n##@##\n#####\n";

    #[test]
    fn introduces_freeze_deadlock_freezes_off_goal_neighbor() {
        let map = Map::from_str(TWO_BOX_CORRIDOR).unwrap();
        // Simulate the upward push: lower box (2,2) -> (2,3); upper box stays.
        let pushed_to = IVector2::new(2, 3);
        let upper = IVector2::new(2, 4);
        let post_push = BoxSet::from_iter(map.dimensions().x, [pushed_to, upper]);

        // Sanity: the just-pushed box sits on a goal, the upper box does not.
        assert!(map[pushed_to].intersects(Tiles::Goal));
        assert!(!map[upper].intersects(Tiles::Goal));

        // The new helper must flag this as a deadlock even though the *pushed*
        // box ended up on a goal — the upper box is frozen off-goal.
        assert!(introduces_freeze_deadlock(&map, pushed_to, &post_push));
    }

    #[test]
    fn introduces_freeze_deadlock_allows_pushed_box_alone_on_goal() {
        // Same map, but no second box: pushing one box onto a goal in an open
        // corridor is fine.
        let xsb = "#####\n##.##\n##$##\n##@##\n#####\n";
        let map = Map::from_str(xsb).unwrap();
        let pushed_to = IVector2::new(2, 3);
        let post_push = BoxSet::from_iter(map.dimensions().x, [pushed_to]);
        assert!(!introduces_freeze_deadlock(&map, pushed_to, &post_push));
    }

    #[test]
    fn introduces_freeze_deadlock_allows_full_group_on_goals() {
        // Same vertical corridor, but the upper box already sits on a goal (`*`).
        // After pushing the lower box up, both frozen boxes are on goals → ok.
        let xsb = "#####\n##*##\n##.##\n##$##\n##@##\n#####\n";
        let map = Map::from_str(xsb).unwrap();
        let pushed_to = IVector2::new(2, 3);
        let upper = IVector2::new(2, 4);
        let post_push = BoxSet::from_iter(map.dimensions().x, [pushed_to, upper]);

        assert!(map[pushed_to].intersects(Tiles::Goal));
        assert!(map[upper].intersects(Tiles::Goal));

        assert!(!introduces_freeze_deadlock(&map, pushed_to, &post_push));
    }

    #[test]
    fn introduces_freeze_deadlock_flags_off_goal_corner() {
        // Pushing a box into a non-goal corner is the canonical freeze deadlock.
        let xsb = "#####\n#  .#\n# $@#\n#####\n";
        let map = Map::from_str(xsb).unwrap();
        // A hypothetical push that lands the box in the bottom-left corner.
        let pushed_to = IVector2::new(1, 1);
        let post_push = BoxSet::from_iter(map.dimensions().x, [pushed_to]);
        assert!(!map[pushed_to].intersects(Tiles::Goal));
        assert!(introduces_freeze_deadlock(&map, pushed_to, &post_push));
    }
}
