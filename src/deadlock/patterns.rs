//! Per-push pattern-based deadlock checks.
//!
//! These are called by the search after each candidate push, on the
//! post-push state. They reject branches that the freeze check alone misses:
//!
//! - [`introduces_2x2_deadlock`]: O(1) check for the four 2×2 blocks containing
//!   the just-pushed box. Catches closed-diagonal clusters where the freeze
//!   recursion would short-circuit at a wall-blocked box.
//! - [`introduces_corral_deadlock`]: a sound but conservative form of PI-corral
//!   pruning. Detects when the push has trapped the box in a region the player
//!   can no longer enter or change.

use crate::{
    box_set::BoxSet, direction::Direction, map::Map, math::IVector2,
    path_finding::reachable_area, tiles::Tiles,
};

/// Returns `true` iff pushing a box to `box_position` closes a 2×2 block of
/// wall-or-box cells in which at least one of the boxes is off-goal.
///
/// Such a block is a *closed-diagonal* deadlock: every box in the 2×2 has its
/// two in-block neighbors blocked (by box or wall on each axis), so none of
/// them can ever be pushed out. The recursive freeze check normally catches
/// this, but it can short-circuit at the first wall-blocked box without
/// visiting the whole frozen cluster, missing off-goal members elsewhere in
/// the same 2×2. This O(1) check fills that gap.
///
/// Only the four 2×2 blocks that contain `box_position` are inspected, since
/// no other 2×2 block could change because of this push.
pub fn introduces_2x2_deadlock(
    map: &Map,
    box_position: IVector2,
    box_positions: &BoxSet,
) -> bool {
    let (x, y) = (box_position.x, box_position.y);

    // The four 2×2 blocks containing (x, y), keyed by their bottom-left corner.
    for &(cx, cy) in &[(x - 1, y - 1), (x, y - 1), (x - 1, y), (x, y)] {
        let cells = [
            IVector2::new(cx, cy),
            IVector2::new(cx + 1, cy),
            IVector2::new(cx, cy + 1),
            IVector2::new(cx + 1, cy + 1),
        ];

        // The block must be fully in bounds and every cell must be wall-or-box.
        let all_blocked = cells.iter().all(|p| {
            map.in_bounds(*p) && (map[*p].intersects(Tiles::Wall) || box_positions.contains(p))
        });
        if !all_blocked {
            continue;
        }

        // At least one box in the block must be off-goal for it to be a deadlock.
        let any_off_goal = cells
            .iter()
            .any(|p| box_positions.contains(p) && !map[*p].intersects(Tiles::Goal));
        if any_off_goal {
            return true;
        }
    }

    false
}

/// Returns `true` if pushing a box to `box_position` traps it in a corral
/// the player can no longer change.
///
/// A *corral* is a maximal connected region of non-wall cells that lies
/// outside the player's reachable area. After a push, the freshly pushed
/// box sits inside such a region (its cell is non-wall and the box itself
/// blocks the player from standing there). If every box on the corral's
/// boundary is unpushable by the player from a reachable cell, the corral
/// is *frozen* — its configuration is permanent. A frozen corral with an
/// off-goal box or an unfilled goal is a deadlock.
///
/// This is a sound but conservative form of PI-corral pruning. It never
/// false-positives — if the corral can still be changed by some push, the
/// check returns `false`. The full Junghanns & Schaeffer rule additionally
/// proves a corral unsolvable by enumerating inward pushes; that extension
/// is left for a later phase.
pub fn introduces_corral_deadlock(
    map: &Map,
    box_position: IVector2,
    box_positions: &BoxSet,
    new_player_position: IVector2,
) -> bool {
    // 1) Player's reachable area in the post-push state. Boxes block.
    let reachable = reachable_area(new_player_position, |p| {
        map.in_bounds(p) && !map[p].intersects(Tiles::Wall) && !box_positions.contains(&p)
    });

    // 2) The corral containing the just-pushed box: connected non-wall cells
    //    outside `reachable`. The box's own cell is non-wall, so it belongs
    //    to the corral.
    let corral = reachable_area(box_position, |p| {
        map.in_bounds(p) && !map[p].intersects(Tiles::Wall) && !reachable.contains(&p)
    });

    // 3) If any box in the corral is currently pushable by the player from
    //    a reachable cell, the corral isn't frozen — bail.
    for &cell in &corral {
        if !box_positions.contains(&cell) {
            continue;
        }
        for dir in Direction::iter() {
            let d: IVector2 = dir.into();
            let dest = cell + d;
            let player_at = cell - d;
            if !map.in_bounds(dest) || map[dest].intersects(Tiles::Wall) {
                continue;
            }
            if box_positions.contains(&dest) {
                continue;
            }
            if reachable.contains(&player_at) {
                return false;
            }
        }
    }

    // 4) Frozen corral. Deadlock iff any box in it is off-goal or any goal
    //    in it is unfilled.
    let off_goal_box = corral
        .iter()
        .any(|p| box_positions.contains(p) && !map[*p].intersects(Tiles::Goal));
    let unfilled_goal = corral
        .iter()
        .any(|p| map[*p].intersects(Tiles::Goal) && !box_positions.contains(p));

    off_goal_box || unfilled_goal
}
