//! Search node representation and successor generation.

use std::cmp::Ordering;

use smallvec::SmallVec;

use crate::{
    deadlock::{introduces_2x2_deadlock, introduces_corral_deadlock, introduces_freeze_deadlock},
    direction::Direction,
    path_finding::reachable_area_with_distances,
    Tiles,
};

use super::state::State;
use super::{Solver, Strategy};

/// Inline storage for successor lists. Most states have ≤8 legal successors;
/// using `SmallVec` avoids the heap allocation in the inner search loop and
/// only spills to the heap for unusually wide states. Each `Node` is ~540 B
/// so 8 inline = ~4.3 KB on the stack — safe even for IDA*'s recursive calls.
type Successors = SmallVec<[Node; 8]>;

/// A node in the search frontier.
///
/// Stores both push and move costs so multiple strategies can share the same
/// successor generator. The state is stored canonically — for push-space
/// strategies the player position has already been normalized.
#[derive(Clone, Eq, Debug)]
pub struct Node {
    /// The canonical Sokoban state at this node.
    pub state: State,
    /// Number of pushes taken to reach this node.
    pub pushes: i32,
    /// Number of player moves (including pushes as 1 move) taken to reach this
    /// node.
    pub moves: i32,
    /// A strategy-dependent priority value for the open list.
    pub priority: i32,
}

impl Node {
    /// Creates a new `Node` and computes its priority according to the solver
    /// strategy.
    ///
    /// - `Fast` uses weighted A* in push space.
    /// - `OptimalPush` uses `pushes + h`.
    /// - `OptimalMove` uses `moves + h`.
    pub fn new(state: State, pushes: i32, moves: i32, solver: &Solver) -> Self {
        let mut state = state;
        // Canonicalize player position for push-space strategies so that
        // equivalent player locations within the same reachable region
        // collapse to a single transposition-table entry.
        if solver.strategy() != Strategy::OptimalMove {
            state.normalize(solver.map());
        }
        let h = state.heuristic(solver);

        let priority = if h == i32::MAX {
            i32::MAX
        } else {
            match solver.strategy() {
                Strategy::Fast => {
                    // Weighted A*: pushes + w*h
                    let w = solver.fast_weight();
                    pushes.saturating_add(((h as f32) * w).ceil() as i32)
                }
                Strategy::Greedy => h,
                Strategy::OptimalPush => pushes.saturating_add(h),
                Strategy::OptimalMove => moves.saturating_add(h),
            }
        };

        Self {
            state,
            pushes,
            moves,
            priority,
        }
    }

    /// Generates successor nodes by enumerating all legal pushes.
    ///
    /// This function performs a single BFS to compute player reachability and
    /// shortest move distances from the current player position. It then tests
    /// each box and direction for:
    ///
    /// 1. Player can reach the behind-square.
    /// 2. Destination square is free.
    /// 3. Destination is not a dead square in the push-distance abstraction.
    /// 4. Resulting configuration does not introduce a freeze deadlock.
    ///
    /// Tunnel macros are applied as forced sequences of pushes through
    /// corridors.
    pub fn successors(&self, solver: &Solver) -> Successors {
        let mut successors: Successors = SmallVec::new();

        // BFS once per expanded node for move-opt edge costs and reachability.
        let dist = reachable_area_with_distances(self.state.player_position, |p| {
            !solver.map()[p].intersects(Tiles::Wall) && !self.state.box_positions.contains(&p)
        });

        for box_position in &self.state.box_positions {
            for push_direction in Direction::iter() {
                let behind = box_position - &push_direction.into();
                let Some(&d_behind) = dist.get(&behind) else {
                    continue;
                };

                let mut new_box_position = box_position + &push_direction.into();

                // Destination must be a free floor tile.
                if !solver.map().in_bounds(new_box_position)
                    || solver.map()[new_box_position].intersects(Tiles::Wall)
                    || self.state.box_positions.contains(&new_box_position)
                {
                    continue;
                }

                // Dead-square pruning based on push-distance abstraction.
                if !solver.lower_bounds().contains_key(&new_box_position) {
                    continue;
                }

                let mut new_player_position = box_position;
                let mut new_pushes = self.pushes + 1;
                let mut new_moves = self.moves + d_behind + 1; // walk behind + push

                // Tunnel macro: optionally compress forced corridor segments.
                while solver.tunnel_macros() && solver.is_tunnel(new_box_position, push_direction) {
                    let next = new_box_position + &push_direction.into();
                    if !solver.map().in_bounds(next)
                        || solver.map()[next].intersects(Tiles::Wall)
                        || self.state.box_positions.contains(&next)
                    {
                        break;
                    }
                    new_player_position = new_box_position;
                    new_box_position = next;
                    new_pushes += 1;
                    new_moves += 1;
                }

                let mut new_box_positions = self.state.box_positions.clone();
                new_box_positions.remove(box_position);
                new_box_positions.insert(new_box_position);

                // Cheap O(1) closed-diagonal (2×2) deadlock check. Catches
                // some clusters the recursive freeze check misses when its
                // recursion short-circuits at a wall-blocked box.
                if introduces_2x2_deadlock(solver.map(), new_box_position, &new_box_positions) {
                    continue;
                }

                // Skip pushes that freeze any off-goal box. Note: the pushed
                // box being on a goal is *not* enough — we must check the whole
                // frozen connected component (see `introduces_freeze_deadlock`).
                if introduces_freeze_deadlock(solver.map(), new_box_position, &new_box_positions) {
                    continue;
                }

                // Conservative PI-corral check, opt-in via `with_corral_pruning`.
                // Costs two extra BFSes per push, so it's off by default and
                // only worth turning on for levels where freeze + 2×2 alone
                // can't keep the search bounded.
                if solver.corral_pruning()
                    && introduces_corral_deadlock(
                        solver.map(),
                        new_box_position,
                        &new_box_positions,
                        new_player_position,
                    )
                {
                    continue;
                }

                successors.push(Node::new(
                    State {
                        player_position: new_player_position,
                        box_positions: new_box_positions,
                    },
                    new_pushes,
                    new_moves,
                    solver,
                ));
            }
        }

        // Sort low-priority-first. For IDA* this lets the depth-first search
        // try the most promising branch first and short-circuit on `Found`;
        // for A* it improves cache locality of recently-pushed heap entries.
        successors.sort_by_key(|n| n.priority);
        successors
    }
}

impl PartialEq for Node {
    // BinaryHeap only consults `Ord`, so this just needs to be consistent with
    // it. Comparing on priority alone is enough — and avoids an expensive
    // full-state equality check on every push/pop.
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority).reverse()
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
