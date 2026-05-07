//! Path-finding utilities used both by the solver search and by replay/UI code.
//!
//! # Module layout
//!
//! - `astar`: grid A* with a Manhattan heuristic. The general-purpose
//!   shortest-path primitive, plus the `player_move_path` convenience for
//!   walking the player to a target cell on a [`crate::Map`].
//! - `reachability`: BFS reachability — `reachable_area`,
//!   `reachable_area_with_distances`, `normalized_area`. Used by the solver
//!   for player canonicalization and deadlock checks.
//! - `box_paths`: replay/UI helpers for finding box paths given a starting
//!   box and following box paths with the player. Not used by the search.

mod astar;
mod box_paths;
mod reachability;

pub use astar::{find_path, player_move_path, FIND_PATH_MAX_EXPANSIONS};
pub use box_paths::{
    box_move_waypoints, construct_box_path, construct_player_path, pushable_boxes,
};
pub use reachability::{normalized_area, reachable_area, reachable_area_with_distances};
