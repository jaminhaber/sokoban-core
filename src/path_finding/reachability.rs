//! Reachability BFS over a grid graph induced by a `can_move` predicate.

use std::collections::VecDeque;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{direction::Direction, math::IVector2};

/// Calculates the reachable area starting from a given position.
///
/// This function performs a breadth-first search to determine all positions
/// that can be reached from the starting position, based on the provided
/// `can_move` function.
pub fn reachable_area(
    position: IVector2,
    can_move: impl Fn(IVector2) -> bool,
) -> FxHashSet<IVector2> {
    // Sokoban maps usually fit in a few dozen cells; pre-sizing avoids the
    // grow-and-rehash chain that dominates BFS cost on small grids.
    let mut reachable_area: FxHashSet<IVector2> =
        FxHashSet::with_capacity_and_hasher(64, Default::default());
    let mut deque = VecDeque::<IVector2>::new();
    deque.push_back(position);

    while let Some(position) = deque.pop_front() {
        if !reachable_area.insert(position) {
            continue;
        }
        for direction in Direction::iter() {
            let neighbor = position + &direction.into();
            if can_move(neighbor) {
                deque.push_back(neighbor);
            }
        }
    }

    reachable_area
}

/// Calculates all reachable positions and their shortest-path distances from
/// `position`.
///
/// This is a standard BFS over the grid graph induced by `can_move`.
/// Distances are measured in number of steps (moves).
///
/// # Returns
///
/// A map `dist[pos] = d` containing all reachable positions and their distances.
///
/// # Complexity
///
/// `O(|reachable tiles|)`.
pub fn reachable_area_with_distances(
    position: IVector2,
    can_move: impl Fn(IVector2) -> bool,
) -> FxHashMap<IVector2, i32> {
    let mut dist: FxHashMap<IVector2, i32> =
        FxHashMap::with_capacity_and_hasher(64, Default::default());
    let mut deque = VecDeque::<IVector2>::new();
    dist.insert(position, 0);
    deque.push_back(position);

    while let Some(p) = deque.pop_front() {
        let d = dist[&p];
        for direction in Direction::iter() {
            let n = p + &direction.into();
            if !can_move(n) {
                continue;
            }
            if dist.contains_key(&n) {
                continue;
            }
            dist.insert(n, d + 1);
            deque.push_back(n);
        }
    }

    dist
}

/// Returns the top-left position in `area`, ordered by `(y, x)`.
///
/// Used by push-space search to canonicalize the player's position within a
/// reachable region: any two states in which the player stands in the same
/// reachable region collapse to the same key.
pub fn normalized_area(area: &FxHashSet<IVector2>) -> Option<IVector2> {
    area.iter()
        .min_by(|a, b| a.y.cmp(&b.y).then_with(|| a.x.cmp(&b.x)))
        .copied()
}
