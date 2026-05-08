//! Grid A* with a Manhattan-distance heuristic.

use std::{cmp::Ordering, collections::BinaryHeap};

use rustc_hash::FxHashMap;

use crate::{direction::Direction, map::Map, math::IVector2};

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
struct Node {
    position: IVector2,
    heuristic: i32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        self.heuristic.cmp(&other.heuristic).reverse()
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Hard cap on A* expansions inside [`find_path`].
///
/// The search is bounded only by `can_move`. If the predicate doesn't clip
/// the grid (returning `true` outside the intended search region), A* would
/// otherwise wander the infinite 2D plane forever. This cap converts that
/// pathological case into a finite-time `None`. For realistic Sokoban maps
/// (≤ ~64×64) the cap is many orders of magnitude larger than needed.
pub const FIND_PATH_MAX_EXPANSIONS: usize = 1_000_000;

/// Finds a shortest path from `from` to `to` using A* with Manhattan distance.
///
/// `can_move` is the only thing that bounds the search region. **The
/// predicate must return `false` outside the intended grid**, otherwise the
/// search would explore the unbounded 2D plane. As a safety net the
/// implementation hard-caps expansions at [`FIND_PATH_MAX_EXPANSIONS`] and
/// returns `None` on overflow rather than hanging.
///
/// In normal use against a [`crate::Map`], pass `|p| map.can_move(p)` (which
/// already checks bounds) and the cap will never fire.
pub fn find_path(
    from: IVector2,
    to: IVector2,
    can_move: impl Fn(IVector2) -> bool,
) -> Option<Vec<IVector2>> {
    let mut open_set = BinaryHeap::new();
    let mut came_from: FxHashMap<IVector2, IVector2> = FxHashMap::default();
    let mut cost: FxHashMap<IVector2, i32> = FxHashMap::default();

    open_set.push(Node {
        position: from,
        heuristic: manhattan_distance(from, to),
    });
    cost.insert(from, 0);

    let mut expansions = 0usize;
    while let Some(node) = open_set.pop() {
        if node.position == to {
            return Some(construct_path(from, to, came_from));
        }

        expansions += 1;
        if expansions >= FIND_PATH_MAX_EXPANSIONS {
            // Likely an unbounded `can_move`; bail rather than hang.
            return None;
        }

        for direction in Direction::iter() {
            let new_position = node.position + &direction.into();
            if !can_move(new_position) {
                continue;
            }

            let new_cost = cost[&node.position] + 1;
            if !cost.contains_key(&new_position) || new_cost < cost[&new_position] {
                cost.insert(new_position, new_cost);
                let priority = new_cost + manhattan_distance(new_position, to);
                open_set.push(Node {
                    position: new_position,
                    heuristic: priority,
                });
                came_from.insert(new_position, node.position);
            }
        }
    }

    None
}

fn construct_path(
    from: IVector2,
    to: IVector2,
    came_from: FxHashMap<IVector2, IVector2>,
) -> Vec<IVector2> {
    let mut path = Vec::new();
    let mut current = to;
    while current != from {
        path.push(current);
        current = came_from[&current];
    }
    path.push(from);
    path.reverse();
    path
}

/// Calculates the path for the player to move from their current position to
/// a target position.
pub fn player_move_path(map: &Map, to: IVector2) -> Option<Vec<Direction>> {
    let path = find_path(map.player_position(), to, |position| map.can_move(position))?;
    Some(convert_path_from_points_to_directions(path))
}

fn convert_path_from_points_to_directions(path: Vec<IVector2>) -> Vec<Direction> {
    path.windows(2)
        .map(|position| Direction::try_from(position[1] - position[0]).unwrap())
        .collect()
}

/// Manhattan distance between two grid positions.
fn manhattan_distance(a: IVector2, b: IVector2) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
