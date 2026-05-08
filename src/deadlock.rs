//! Deadlock detection — both static analysis (offline, from the map alone)
//! and per-push runtime checks (called by the search after each candidate
//! move).
//!
//! # Module layout
//!
//! - `freeze`: the recursive freeze rule. A box is frozen iff both axes are
//!   blocked by walls or other frozen boxes; a frozen group with any off-goal
//!   member is a deadlock.
//! - `static_analysis`: properties computed once from the map alone — corner /
//!   groove dead squares, useless dead-end floors, fully-enclosed box clusters.
//! - `patterns`: per-push pattern checks — closed 2×2 blocks and the
//!   conservative PI-corral check. These run after every candidate push to
//!   prune branches the freeze recursion would miss.

mod freeze;
mod patterns;
mod static_analysis;

pub use freeze::{calculate_useless_boxes, introduces_freeze_deadlock, is_freeze_deadlock};
pub use patterns::{introduces_2x2_deadlock, introduces_corral_deadlock};
pub use static_analysis::{
    calculate_static_deadlocks, calculate_useless_floors, is_static_deadlock,
};

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::str::FromStr;

    use super::*;
    use crate::{box_set::BoxSet, map::Map, math::IVector2, tiles::Tiles};

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
        let pushed_to = IVector2::new(2, 3);
        let upper = IVector2::new(2, 4);
        let post_push = BoxSet::from_iter(map.dimensions().x, [pushed_to, upper]);

        assert!(map[pushed_to].intersects(Tiles::Goal));
        assert!(!map[upper].intersects(Tiles::Goal));

        assert!(introduces_freeze_deadlock(&map, pushed_to, &post_push));
    }

    #[test]
    fn introduces_freeze_deadlock_allows_pushed_box_alone_on_goal() {
        let xsb = "#####\n##.##\n##$##\n##@##\n#####\n";
        let map = Map::from_str(xsb).unwrap();
        let pushed_to = IVector2::new(2, 3);
        let post_push = BoxSet::from_iter(map.dimensions().x, [pushed_to]);
        assert!(!introduces_freeze_deadlock(&map, pushed_to, &post_push));
    }

    #[test]
    fn introduces_freeze_deadlock_allows_full_group_on_goals() {
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
        let xsb = "#####\n#  .#\n# $@#\n#####\n";
        let map = Map::from_str(xsb).unwrap();
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
        let boxes = BoxSet::from_iter(
            map.dimensions().x,
            [
                IVector2::new(3, 1),
                IVector2::new(4, 1),
                IVector2::new(3, 2),
                IVector2::new(4, 2),
            ],
        );
        assert!(introduces_2x2_deadlock(&map, IVector2::new(4, 2), &boxes));
    }

    #[test]
    fn introduces_2x2_deadlock_all_on_goals_is_ok() {
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
        let map = Map::from_str(PLAYGROUND).unwrap();
        let boxes = BoxSet::from_iter(map.dimensions().x, [IVector2::new(1, 1)]);
        assert!(introduces_2x2_deadlock(&map, IVector2::new(1, 1), &boxes));
    }

    #[test]
    fn introduces_corral_deadlock_flags_off_goal_corner() {
        let xsb = "######\n# $@.#\n######\n";
        let map = Map::from_str(xsb).unwrap();
        let pushed_to = IVector2::new(1, 1);
        let new_player = IVector2::new(2, 1);
        let post_push = BoxSet::from_iter(map.dimensions().x, [pushed_to]);
        assert!(introduces_corral_deadlock(
            &map, pushed_to, &post_push, new_player
        ));
    }

    #[test]
    fn introduces_corral_deadlock_passes_when_box_still_pushable() {
        let xsb = "########\n#@ $  .#\n########\n";
        let map = Map::from_str(xsb).unwrap();
        let pushed_to = IVector2::new(4, 1);
        let new_player = IVector2::new(3, 1);
        let post_push = BoxSet::from_iter(map.dimensions().x, [pushed_to]);
        assert!(!introduces_corral_deadlock(
            &map, pushed_to, &post_push, new_player
        ));
    }

    #[test]
    fn introduces_corral_deadlock_allows_solved_frozen_pocket() {
        let xsb = "######\n# $@ #\n#.   #\n######\n";
        let map = Map::from_str(xsb).unwrap();
        let pushed_to = IVector2::new(1, 1);
        let new_player = IVector2::new(2, 1);
        let post_push = BoxSet::from_iter(map.dimensions().x, [pushed_to]);

        assert!(map[pushed_to].intersects(Tiles::Goal));

        assert!(!introduces_corral_deadlock(
            &map, pushed_to, &post_push, new_player
        ));
    }

    /// Three boxes A-B-C in a horizontal row, walls top and bottom of all
    /// three. After pushing A, the post-push state has A on goal, B on goal,
    /// and C off goal. The whole row is mutually frozen (every box's
    /// horizontal axis is blocked by frozen neighbors, and every box's
    /// vertical axis is blocked by walls), so this is a freeze deadlock —
    /// C is permanently stuck off-goal.
    ///
    /// Regression test: a previous short-circuit in `is_freeze_deadlock`
    /// stopped recursing into the right-hand neighbor as soon as the
    /// left-hand neighbor returned true, leaving C out of `visited`.
    /// `introduces_freeze_deadlock` then walked the partial cluster, missed
    /// the off-goal C, and returned false.
    ///
    /// Layout (XSB top → bottom; player on the right side of the row).
    /// The parsed map has only A and B as boxes-on-goals; the third box C
    /// is supplied via the post-push `BoxSet` (the cell at C is just floor
    /// in the parsed map, off-goal).
    /// ```text
    /// ##########
    /// # ###### #     walls directly above A, B, C
    /// #  ** @  #     A on goal, B on goal, C is the floor at (5, 2)
    /// # ###### #     walls directly below A, B, C
    /// ##########
    /// ```
    #[test]
    fn introduces_freeze_deadlock_walks_full_cluster_for_off_goal_member() {
        let xsb = "##########\n\
                   # ###### #\n\
                   #  ** @  #\n\
                   # ###### #\n\
                   ##########\n";
        let map = Map::from_str(xsb).unwrap();
        let a = IVector2::new(3, 2);
        let b = IVector2::new(4, 2);
        let c = IVector2::new(5, 2);

        // Sanity: A and B are on goals, C is not.
        assert!(map[a].intersects(Tiles::Goal));
        assert!(map[b].intersects(Tiles::Goal));
        assert!(!map[c].intersects(Tiles::Goal));

        // Hypothetical post-push state: three boxes in a row at A, B, C.
        let post_push = BoxSet::from_iter(map.dimensions().x, [a, b, c]);

        // C is off-goal and frozen, so the push introduces a freeze deadlock.
        assert!(introduces_freeze_deadlock(&map, a, &post_push));
    }

    /// `is_freeze_deadlock` must not panic when called on a malformed map
    /// without a wall perimeter. A box at the very edge of the map has
    /// out-of-bounds neighbors; indexing the map at those positions panics.
    /// The function is `pub`, so callers can construct such maps via
    /// `Map::with_dimensions`. Treat out-of-bounds as a wall — the axis is
    /// blocked by the map edge with the same semantics as a wall.
    #[test]
    fn is_freeze_deadlock_handles_out_of_bounds_neighbors() {
        let dimensions = IVector2::new(3, 3);
        let map = Map::with_dimensions(dimensions);

        // Corner box: both vertical and horizontal axes have an out-of-bounds
        // neighbor. With OOB-as-wall, both axes are "blocked by the edge"
        // and the box is reported as frozen — and crucially, no panic.
        let corner = IVector2::new(0, 0);
        let corner_set = BoxSet::from_iter(dimensions.x, [corner]);
        let mut visited = HashSet::new();
        assert!(is_freeze_deadlock(&map, corner, &corner_set, &mut visited));

        // Center box: both axes are open (in-bounds floor on both sides),
        // so the box is not frozen.
        let center = IVector2::new(1, 1);
        let center_set = BoxSet::from_iter(dimensions.x, [center]);
        let mut visited = HashSet::new();
        assert!(!is_freeze_deadlock(&map, center, &center_set, &mut visited));
    }
}
