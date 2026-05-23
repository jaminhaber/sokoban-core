//! Precomputation for the push-distance abstraction and tunnel-macro detection.
//!
//! These run once at first access via [`super::Solver`]'s `OnceCell` fields.
//! Both functions are pure — they take a [`Map`] (and for tunnels, the
//! already-computed lower bounds) and return their result. No solver state
//! is borrowed beyond what's passed in.

use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

use crate::{direction::Direction, map::Map, math::IVector2, Tiles};

/// Computes push distances in an admissible abstraction by reverse BFS from
/// each goal cell.
///
/// The push graph treats the board as empty (no boxes) and admits a push
/// from `prev` to `cur = prev + dir` iff:
///
/// - `prev` is not a wall
/// - `cur` is not a wall
/// - `prev - dir` is not a wall (the player can stand behind to push)
///
/// Returns `(lower_bounds, distance_matrix)` where:
///
/// - `lower_bounds[pos]` is the minimum push distance from `pos` to any goal
///   (cells absent from this map have no path to any goal — i.e. they are dead
///   squares for placing a box).
/// - `distance_matrix[pos][goal]` is the minimum push distance from `pos` to
///   that specific `goal`. Used by the bipartite-matching heuristic.
pub(super) fn compute_push_distances(
    map: &Map,
) -> (
    FxHashMap<IVector2, i32>,
    FxHashMap<IVector2, FxHashMap<IVector2, i32>>,
) {
    let is_free = |p: IVector2| -> bool { map.in_bounds(p) && !map[p].intersects(Tiles::Wall) };

    let mut distance_matrix: FxHashMap<IVector2, FxHashMap<IVector2, i32>> = FxHashMap::default();

    for &goal in map.goal_positions().iter() {
        if !is_free(goal) {
            continue;
        }

        let mut dist_to_goal: FxHashMap<IVector2, i32> = FxHashMap::default();
        let mut q = VecDeque::new();

        dist_to_goal.insert(goal, 0);
        q.push_back(goal);

        while let Some(cur) = q.pop_front() {
            let dcur = dist_to_goal[&cur];

            // Reverse edges: predecessor `prev` such that `prev + dir = cur`.
            for dir in Direction::iter() {
                let prev = cur - &dir.into();
                let behind = prev - &dir.into();

                if !is_free(prev) || !is_free(behind) {
                    continue;
                }
                if dist_to_goal.contains_key(&prev) {
                    continue;
                }

                dist_to_goal.insert(prev, dcur + 1);
                q.push_back(prev);
            }
        }

        for (pos, d) in dist_to_goal {
            distance_matrix.entry(pos).or_default().insert(goal, d);
        }
    }

    let mut lower_bounds: FxHashMap<IVector2, i32> = FxHashMap::default();
    for (pos, gm) in &distance_matrix {
        if let Some(best) = gm.values().min().copied() {
            lower_bounds.insert(*pos, best);
        }
    }

    (lower_bounds, distance_matrix)
}

/// Detects static corridors where the box's continuation is forced by walls
/// on both perpendicular sides, both at the box position and one step ahead.
///
/// Returns the set of `(box_position, push_direction)` pairs that should be
/// followed by an automatic next push. The detector is conservative: it only
/// marks a step as a tunnel if the destination is reachable in the abstraction
/// (i.e. present in `lower_bounds`), avoiding macros into unreachable
/// corridors.
pub(super) fn compute_tunnels(
    map: &Map,
    lower_bounds: &FxHashMap<IVector2, i32>,
) -> FxHashSet<(IVector2, Direction)> {
    let mut tunnels = FxHashSet::default();

    let is_free = |p: IVector2| -> bool { map.in_bounds(p) && !map[p].intersects(Tiles::Wall) };

    for x in 0..map.dimensions().x {
        for y in 0..map.dimensions().y {
            let p = IVector2::new(x, y);
            if !is_free(p) || map[p].intersects(Tiles::Goal) {
                continue;
            }

            for dir in Direction::iter() {
                let forward = p + &dir.into();
                if !is_free(forward) || map[forward].intersects(Tiles::Goal) {
                    continue;
                }

                let (l, r) = dir.perpendiculars();
                let lp = p + &l.into();
                let rp = p + &r.into();
                if !map.in_bounds(lp)
                    || !map.in_bounds(rp)
                    || !map[lp].intersects(Tiles::Wall)
                    || !map[rp].intersects(Tiles::Wall)
                {
                    continue;
                }

                // The forward cell must also be corridor-like (walls on both perpendicular
                // sides).
                let flp = forward + &l.into();
                let frp = forward + &r.into();
                if !map.in_bounds(flp)
                    || !map.in_bounds(frp)
                    || !map[flp].intersects(Tiles::Wall)
                    || !map[frp].intersects(Tiles::Wall)
                {
                    continue;
                }

                // Don't macro into squares the abstraction marks dead.
                if !lower_bounds.contains_key(&forward) {
                    continue;
                }

                tunnels.insert((forward, dir));
            }
        }
    }

    tunnels
}
