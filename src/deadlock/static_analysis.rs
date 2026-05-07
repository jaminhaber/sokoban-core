//! Static deadlock analysis — properties computed once from the map alone,
//! independent of the player's position.
//!
//! These functions identify cells that are "dead" in the abstract sense:
//!   - Floor cells that, if a box were placed on them, would be permanently stuck.
//!   - Floor cells that the player can never usefully visit (single-exit dead ends).
//!
//! Used during map normalization (collapsing unreachable corridors into walls)
//! and as a complement to runtime deadlock checks.

use std::collections::{HashSet, VecDeque};

use crate::{box_set::BoxSet, direction::Direction, map::Map, math::IVector2, tiles::Tiles};

/// Returns `true` if the given box and its connected box-cluster occupy a
/// configuration where every cluster member has all four directions blocked
/// by walls or other cluster members.
///
/// This is a *strong* freeze condition: walls AND frozen-box neighbors must
/// jointly block all four directions, not just both axes (which is the
/// weaker [`super::freeze::is_freeze_deadlock`] condition). Useful for
/// detecting fully-enclosed box clusters.
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

/// Calculates static deadlock positions independent of the player's position.
///
/// This function returns an **incomplete** set of dead positions independent
/// of the player's position. Any box pushed to a point in the set will cause
/// a deadlock, regardless of the player's position. Detects two patterns:
///
/// - Corner cells (two adjacent walls) that are not goals.
/// - Groove cells along walls that don't lead to a goal.
pub fn calculate_static_deadlocks(map: &Map) -> HashSet<IVector2> {
    let mut dead_positions = HashSet::new();
    for x in 1..map.dimensions().x - 1 {
        for y in 1..map.dimensions().y - 1 {
            let position = IVector2::new(x, y);
            // Skip walls and goals.
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

                // Is this position a corner (two adjacent walls)?
                if !(map[neighbor[0]].intersects(Tiles::Wall)
                    && map[neighbor[1]].intersects(Tiles::Wall))
                {
                    continue;
                }
                dead_positions.insert(position);

                // Walk along the wall to find groove cells that share the
                // corner's fate (no goal between them and the corner).
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

/// Calculates the positions of "useless" floor cells — floor cells the player
/// can only enter and exit through a single neighbor (dead-end corridors).
/// Used during map normalization to fill those cells with walls.
pub fn calculate_useless_floors(mut map: Map) -> HashSet<IVector2> {
    let mut useless_floors = HashSet::new();

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
        // A "useless" floor has exactly one non-wall neighbor.
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
        if let Some(free_neighbor) = neighbor_floor {
            useless_floors.insert(position);
            map[position].remove(Tiles::Floor);
            map[position].insert(Tiles::Wall);
            // The freed neighbor may itself become useless; recheck it.
            if map[free_neighbor] == Tiles::Floor && !map[free_neighbor].intersects(Tiles::Wall) {
                unchecked_floors.push_back(free_neighbor);
            }
        }
    }

    useless_floors
}
