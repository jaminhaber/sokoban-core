//! Utilities for deadlocks detection.

use std::collections::{HashSet, VecDeque};

use crate::{
    box_set::BoxSet, direction::Direction, map::Map, math::IVector2,
    path_finding::reachable_area, tiles::Tiles,
};

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

    /// Open 8×5 inner area with a single starting box and goal far from where
    /// the tests place hypothetical 2×2 box clusters. Used as a wall-free
    /// playground so the 2×2 helper sees only boxes and floor.
    const PLAYGROUND: &str = "##########\n\
                              #        #\n\
                              #        #\n\
                              #        #\n\
                              #        #\n\
                              #@$  .   #\n\
                              ##########\n";

    #[test]
    fn introduces_2x2_deadlock_pure_box_block_off_goal() {
        let map = Map::from_str(PLAYGROUND).unwrap();
        // Four boxes at (3,1)..(4,2) on plain floor, none on goals.
        let boxes = BoxSet::from_iter(
            map.dimensions().x,
            [
                IVector2::new(3, 1),
                IVector2::new(4, 1),
                IVector2::new(3, 2),
                IVector2::new(4, 2),
            ],
        );
        // Pretend the most recent push landed on (4, 2).
        assert!(introduces_2x2_deadlock(&map, IVector2::new(4, 2), &boxes));
    }

    #[test]
    fn introduces_2x2_deadlock_all_on_goals_is_ok() {
        // The 2×2 at (4,2)..(5,3) is all `*` — boxes already on goals.
        let xsb = "############\n\
                   #          #\n\
                   #@         #\n\
                   #          #\n\
                   #   **     #\n\
                   #   **     #\n\
                   #          #\n\
                   ############\n";
        let map = Map::from_str(xsb).unwrap();
        let boxes = BoxSet::from_iter(
            map.dimensions().x,
            [
                IVector2::new(4, 2),
                IVector2::new(5, 2),
                IVector2::new(4, 3),
                IVector2::new(5, 3),
            ],
        );
        assert!(!introduces_2x2_deadlock(&map, IVector2::new(5, 3), &boxes));
    }

    #[test]
    fn introduces_2x2_deadlock_three_boxes_one_wall() {
        // Internal wall at (3, 3); three test boxes complete the 2×2 at
        // corner (2,2)..(3,3) — that's 3 boxes + 1 wall = closed.
        let xsb = "##########\n\
                   #        #\n\
                   #  #     #\n\
                   #        #\n\
                   #@$.     #\n\
                   ##########\n";
        let map = Map::from_str(xsb).unwrap();
        let boxes = BoxSet::from_iter(
            map.dimensions().x,
            [
                IVector2::new(2, 2),
                IVector2::new(3, 2),
                IVector2::new(2, 3),
            ],
        );
        assert!(introduces_2x2_deadlock(&map, IVector2::new(3, 2), &boxes));
    }

    #[test]
    fn introduces_2x2_deadlock_open_cell_is_ok() {
        // Three boxes don't close a 2×2 — the fourth cell is floor.
        let map = Map::from_str(PLAYGROUND).unwrap();
        let boxes = BoxSet::from_iter(
            map.dimensions().x,
            [
                IVector2::new(3, 1),
                IVector2::new(4, 1),
                IVector2::new(3, 2),
            ],
        );
        assert!(!introduces_2x2_deadlock(&map, IVector2::new(3, 2), &boxes));
    }

    #[test]
    fn introduces_2x2_deadlock_corner_with_three_walls() {
        // (1,1) sits in the bottom-left interior corner — three of the four
        // cells of the 2×2 anchored at (0,0) are walls.
        let map = Map::from_str(PLAYGROUND).unwrap();
        let boxes = BoxSet::from_iter(map.dimensions().x, [IVector2::new(1, 1)]);
        assert!(introduces_2x2_deadlock(&map, IVector2::new(1, 1), &boxes));
    }

    #[test]
    fn introduces_corral_deadlock_flags_off_goal_corner() {
        // After pushing left, the box lands in the bottom-left corner. The
        // corral is the single cell {(1,1)}, frozen, off-goal — deadlock.
        let xsb = "######\n# $@.#\n######\n";
        let map = Map::from_str(xsb).unwrap();
        let pushed_to = IVector2::new(1, 1);
        let new_player = IVector2::new(2, 1);
        let post_push = BoxSet::from_iter(map.dimensions().x, [pushed_to]);
        assert!(introduces_corral_deadlock(&map, pushed_to, &post_push, new_player));
    }

    #[test]
    fn introduces_corral_deadlock_passes_when_box_still_pushable() {
        // The just-pushed box is in an open corridor and has free cells in
        // multiple directions; the corral isn't frozen.
        let xsb = "########\n#@ $  .#\n########\n";
        let map = Map::from_str(xsb).unwrap();
        // Simulate pushing right: box (3,1) → (4,1); player ends at (3,1).
        let pushed_to = IVector2::new(4, 1);
        let new_player = IVector2::new(3, 1);
        let post_push = BoxSet::from_iter(map.dimensions().x, [pushed_to]);
        assert!(!introduces_corral_deadlock(
            &map,
            pushed_to,
            &post_push,
            new_player
        ));
    }

    #[test]
    fn introduces_corral_deadlock_allows_solved_frozen_pocket() {
        // The box ends up on a goal in a single-cell corner. The corral is
        // frozen but fully solved — not a deadlock.
        let xsb = "######\n# $@ #\n#.   #\n######\n";
        let map = Map::from_str(xsb).unwrap();
        // Hand-simulate: push box (2,2) down, then left. After the second
        // push the box sits at (1,1) — the goal. Player ends at (2,1).
        let pushed_to = IVector2::new(1, 1);
        let new_player = IVector2::new(2, 1);
        let post_push = BoxSet::from_iter(map.dimensions().x, [pushed_to]);

        // Sanity: pushed-to is the goal cell.
        assert!(map[pushed_to].intersects(Tiles::Goal));

        assert!(!introduces_corral_deadlock(
            &map,
            pushed_to,
            &post_push,
            new_player
        ));
    }
}
