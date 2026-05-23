//! Box-movement helpers used by replay/UI code, not by the solver search.
//!
//! Given a starting box position, [`box_move_waypoints`] computes every
//! `(target_position, push_direction)` reachable via a sequence of legal
//! pushes. [`construct_box_path`] turns that table into a concrete sequence
//! of box positions and [`construct_player_path`] walks the player along
//! that sequence using [`super::astar::find_path`].

use std::collections::{hash_map::Entry, HashMap, VecDeque};

use rustc_hash::FxHashSet;

use crate::{direction::Direction, map::Map, math::IVector2, tiles::Tiles};

use super::astar::find_path;
use super::reachability::reachable_area;

/// Calculates the waypoints for a box to move from its current position to
/// every position it can reach via legal pushes.
// TODO:
// 1. Make this generic over the cost metric so callers can prefer move-count
//    over push-count when finding "best" box paths.
// 2. Computing the full player-reachable area on every iteration is expensive.
//    Possible optimizations:
//    - Precompute graph cut vertices to quickly decide whether the player
//      can reach a particular side of a box. This skips the per-step BFS
//      but doesn't yield concrete paths, so it can't be used for move-optimal
//      pathfinding. Reference: <http://sokoban.ws/blog/?p=843>
//    - Use `find_path` and seed each new search with the previous search's
//      endpoint — the Manhattan heuristic stays small and most calls are
//      faster.
//    - Maintain the player-reachable area incrementally across iterations.
pub fn box_move_waypoints(
    map: &Map,
    initial_box_position: IVector2,
) -> HashMap<(IVector2, Direction), u64> {
    debug_assert!(
        map.box_positions().contains(&initial_box_position),
        "box position does not exist"
    );

    let mut deque = VecDeque::new();
    let mut path: HashMap<(IVector2, Direction), u64> = HashMap::new();

    let player_reachable_area = reachable_area(map.player_position(), |position| {
        position == initial_box_position || map.can_move(position)
    });
    for direction in Direction::iter() {
        if !player_reachable_area.contains(&(initial_box_position - &direction.into())) {
            continue;
        }
        let node = (initial_box_position, direction, 0);
        deque.push_back(node);
    }

    while let Some((box_position, push_direction, cost)) = deque.pop_front() {
        let player_position = box_position - &push_direction.into();
        let player_reachable_area = reachable_area(player_position, |position| {
            (position == initial_box_position || map.can_move(position)) && position != box_position
        });

        let new_cost = cost + 1;
        for push_direction in Direction::iter() {
            let new_box_position = box_position + &push_direction.into();
            if !(new_box_position == initial_box_position || map.can_move(new_box_position)) {
                continue;
            }
            let new_player_position = box_position - &push_direction.into();
            if !player_reachable_area.contains(&new_player_position) {
                continue;
            }

            match path.entry((new_box_position, push_direction)) {
                Entry::Occupied(_) => continue,
                Entry::Vacant(entry) => {
                    entry.insert(new_cost);
                }
            }
            deque.push_back((new_box_position, push_direction, new_cost));
        }
    }

    path
}

/// Creates a path for a box to move from its current position to a target
/// position, using a waypoints table from [`box_move_waypoints`].
pub fn construct_box_path(
    from: IVector2,
    to: IVector2,
    waypoints: &HashMap<(IVector2, Direction), u64>,
) -> Vec<IVector2> {
    let mut path = Vec::new();
    let (mut current, mut direction, mut cost) = Direction::iter()
        .filter_map(|push_direction| {
            waypoints
                .get(&(to, push_direction))
                .map(|&cost| (to, push_direction, cost))
        })
        .min_by_key(|&(_, _, cost)| cost)
        .unwrap_or_else(|| panic!("no box waypoint reaches {to} from {from}"));

    while current != from {
        path.push(current);

        let predecessor = current - &direction.into();
        if predecessor == from {
            current = predecessor;
            continue;
        }

        let previous_cost = cost
            .checked_sub(1)
            .expect("non-initial box waypoint must have positive cost");
        let previous_direction = Direction::iter()
            .find(|&push_direction| {
                waypoints.get(&(predecessor, push_direction)) == Some(&previous_cost)
            })
            .unwrap_or_else(|| {
                panic!(
                    "box waypoint chain from {from} to {to} is missing {predecessor} at cost {previous_cost}"
                )
            });

        current = predecessor;
        direction = previous_direction;
        cost = previous_cost;
    }
    path.push(from);
    path.reverse();
    path
}

/// Constructs a player path that follows a given box path — the moves the
/// player has to make in order to push the box from cell to cell.
pub fn construct_player_path(
    map: &Map,
    mut player_position: IVector2,
    box_path: &[IVector2],
) -> Vec<IVector2> {
    let mut path = Vec::new();
    let initial_box_position = *box_path.first().unwrap();
    for box_positions in box_path.windows(2) {
        let direction = box_positions[1] - box_positions[0];
        let new_player_position = box_positions[0] - direction;
        path.append(
            &mut find_path(player_position, new_player_position, |position| {
                (position == initial_box_position
                    || !map[position].intersects(Tiles::Wall | Tiles::Box))
                    && position != box_positions[0]
            })
            .unwrap(),
        );
        player_position = box_positions[0];
    }
    path.push(player_position);
    path
}

/// Returns the set of box positions the player can reach and push from at
/// least one direction.
pub fn pushable_boxes(map: &Map) -> FxHashSet<IVector2> {
    let player_reachable_area =
        reachable_area(map.player_position(), |position| map.can_move(position));
    let mut pushable_boxes: FxHashSet<IVector2> = FxHashSet::default();
    for box_position in map.box_positions() {
        for direction in Direction::iter() {
            let player_position = box_position - &direction.into();
            let new_box_position = box_position + &direction.into();
            if player_reachable_area.contains(&player_position) && map.can_move(new_box_position) {
                pushable_boxes.insert(box_position);
                break;
            }
        }
    }
    pushable_boxes
}
