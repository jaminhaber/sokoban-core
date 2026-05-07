//! A solver for the Sokoban problem.
//!
//! The public entry point is [`Solver`]. Construct one with a [`Map`] and a
//! [`Strategy`], optionally configure it with builder methods (`with_*`),
//! then call [`Solver::a_star_search`] or [`Solver::ida_star_search`].
//!
//! # Module layout
//!
//! - This file: the [`Solver`] struct, builder methods, and the A* / IDA*
//!   search loops that read directly from the solver's private state.
//! - [`strategy`]: the [`Strategy`] and [`Terminator`] enums plus the
//!   internal `TerminatorInner` and `IdaDfsResult` types.
//! - [`preprocess`]: pure functions that compute the push-distance
//!   abstraction (lower bounds + distance matrix) and the tunnel-macro table.
//! - [`reconstruct`]: walks the parent chain of canonical states and emits a
//!   valid player action sequence, simulating forward from the real initial
//!   state to find walking paths between pushes.

mod node;
mod preprocess;
mod reconstruct;
mod state;
mod strategy;

pub use strategy::{Strategy, Terminator};

use node::Node;
use state::{State, StateKey};
use strategy::{IdaDfsResult, TerminatorInner};

use crate::{
    box_set::BoxSet, direction::Direction, math::IVector2, Actions, Map, SearchError,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    cell::{OnceCell, RefCell},
    collections::BinaryHeap,
};

/// Maximum number of heuristic-cache entries before a wholesale flush. At
/// ~520 bytes per entry (BoxSet + i32 + map overhead), 1M caps memory near
/// 520 MB. Tune up if you have memory headroom and search hard levels.
const HEURISTIC_CACHE_LIMIT: usize = 1_000_000;

/// A solver for the Sokoban problem.
///
/// This solver operates primarily in **push space** (each edge is a push).
/// For `OptimalMove`, push edges are weighted by the player's distance to the
/// behind-square plus 1 for the push.
#[derive(Clone, Debug)]
pub struct Solver {
    map: Map,
    strategy: Strategy,

    // Weighted A* parameter for Strategy::Fast.
    fast_weight: f32,

    // Whether to compress forced corridor pushes using tunnel macros.
    tunnel_macros: bool,

    // Whether to run the conservative PI-corral deadlock check on each push.
    // Off by default: the per-push cost (two BFSes) outweighs the pruning win
    // on small/easy levels. Enable on hard levels where freeze and 2×2 alone
    // are not enough to keep the search bounded.
    corral_pruning: bool,

    // lower_bounds[pos] = minimum pushes from pos to any goal in the abstraction
    lower_bounds: OnceCell<FxHashMap<IVector2, i32>>,

    // distance_matrix[pos][goal] = minimum pushes from pos to that goal in the abstraction
    distance_matrix: OnceCell<FxHashMap<IVector2, FxHashMap<IVector2, i32>>>,

    // tunnel macros keyed by (box_position, push_direction)
    tunnels: OnceCell<FxHashSet<(IVector2, Direction)>>,

    // Heuristic memoization keyed on box configuration. The matching heuristic
    // depends only on box positions (player position is irrelevant), so the
    // same h-value applies to every state with the same `BoxSet`. A* and IDA*
    // visit each `BoxSet` many times, often with different player positions or
    // along different paths; this avoids paying the O(n·2ⁿ) matching cost on
    // every revisit. Wrapped in `RefCell` because `State::heuristic` is called
    // through `&Solver`.
    heuristic_cache: RefCell<FxHashMap<BoxSet, i32>>,

    terminator: Terminator,
}

// =====================================================================
// Construction & builder methods.
// =====================================================================

impl Solver {
    /// Creates a new `Solver`.
    ///
    /// Expensive preprocessing (push distances, dead squares, tunnels) is
    /// computed lazily the first time it is needed.
    pub fn new(map: Map, strategy: Strategy) -> Self {
        Self {
            map,
            strategy,
            fast_weight: 2.0,
            tunnel_macros: false,
            corral_pruning: false,
            lower_bounds: OnceCell::new(),
            distance_matrix: OnceCell::new(),
            tunnels: OnceCell::new(),
            heuristic_cache: RefCell::new(FxHashMap::default()),
            terminator: Terminator::None,
        }
    }

    /// Enables or disables tunnel macro compression.
    ///
    /// When enabled, successor generation will repeatedly push boxes through
    /// detected tunnel corridors as a single successor, reducing search depth.
    /// Keep this off if your tunnel detector is overly aggressive for some
    /// level set.
    pub fn with_tunnel_macros(mut self, enabled: bool) -> Self {
        self.tunnel_macros = enabled;
        self
    }

    /// Returns whether tunnel macro compression is enabled.
    pub fn tunnel_macros(&self) -> bool {
        self.tunnel_macros
    }

    /// Enables or disables the conservative PI-corral deadlock check.
    ///
    /// When enabled, after each push the solver checks whether the just-pushed
    /// box ended up in a connected region the player can no longer enter and
    /// can no longer change. Such *frozen corrals* are deadlocks if they
    /// aren't fully solved.
    ///
    /// Off by default. The check costs two extra BFSes per push, which on
    /// easy levels outweighs the pruning benefit. Enable it for hard levels
    /// where the search would otherwise blow up.
    pub fn with_corral_pruning(mut self, enabled: bool) -> Self {
        self.corral_pruning = enabled;
        self
    }

    /// Returns whether corral pruning is enabled.
    pub fn corral_pruning(&self) -> bool {
        self.corral_pruning
    }

    /// Sets the terminator for the solver.
    pub fn with_terminator(mut self, terminator: Terminator) -> Self {
        self.terminator = terminator;
        self
    }

    /// Sets the weight used by `Strategy::Fast` (weighted A*).
    ///
    /// `weight = 1.0` behaves like optimal A* in push space.
    /// Larger values tend to find solutions faster but may sacrifice optimality.
    /// Values below 1.0 are clamped to 1.0.
    pub fn with_fast_weight(mut self, weight: f32) -> Self {
        self.fast_weight = weight.max(1.0);
        self
    }

    /// Returns the terminator.
    pub fn terminator(&self) -> Terminator {
        self.terminator
    }

    /// Returns the map.
    pub fn map(&self) -> &Map {
        &self.map
    }

    /// Returns the strategy.
    pub fn strategy(&self) -> Strategy {
        self.strategy
    }

    /// Returns the Fast-mode heuristic weight.
    pub fn fast_weight(&self) -> f32 {
        self.fast_weight
    }
}

// =====================================================================
// Heuristic cache (used by `State::heuristic` through `&Solver`).
// =====================================================================

impl Solver {
    /// Returns the cached heuristic for `boxes`, if any.
    pub(crate) fn cached_heuristic(&self, boxes: &BoxSet) -> Option<i32> {
        self.heuristic_cache.borrow().get(boxes).copied()
    }

    /// Records `h` as the heuristic for `boxes` in the per-solver cache.
    ///
    /// The cache is hard-capped at [`HEURISTIC_CACHE_LIMIT`] entries. Once
    /// full it is cleared rather than evicted entry-by-entry — losing some
    /// hits is acceptable, but unbounded growth on hard searches is not.
    pub(crate) fn cache_heuristic(&self, boxes: BoxSet, h: i32) {
        let mut cache = self.heuristic_cache.borrow_mut();
        if cache.len() >= HEURISTIC_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(boxes, h);
    }
}

// =====================================================================
// Lazy preprocessing accessors.
// =====================================================================

impl Solver {
    /// Returns lower bounds (dead-square / min-to-any-goal) computed from push distances.
    pub fn lower_bounds(&self) -> &FxHashMap<IVector2, i32> {
        self.lower_bounds
            .get_or_init(|| preprocess::compute_push_distances(&self.map).0)
    }

    /// Returns the push distance matrix used by the matching heuristic.
    pub fn distance_matrix(&self) -> &FxHashMap<IVector2, FxHashMap<IVector2, i32>> {
        self.distance_matrix
            .get_or_init(|| preprocess::compute_push_distances(&self.map).1)
    }

    /// Returns the tunnel macro table.
    pub fn tunnels(&self) -> &FxHashSet<(IVector2, Direction)> {
        self.tunnels
            .get_or_init(|| preprocess::compute_tunnels(&self.map, self.lower_bounds()))
    }

    /// Returns whether `(box_pos, dir)` is a tunnel macro step.
    pub fn is_tunnel(&self, box_pos: IVector2, dir: Direction) -> bool {
        self.tunnels().contains(&(box_pos, dir))
    }

    /// Canonicalizes `state` and returns a transposition-table key for it.
    ///
    /// Push-space strategies normalize the player position before keying so
    /// that two states with the player in different cells of the same
    /// reachable region are treated as the same state. Move-optimal search
    /// keeps the exact player position, since walking distance from there
    /// affects future move cost.
    ///
    /// The returned [`StateKey`] caches a 64-bit hash for fast hash-map
    /// lookups while comparing the full state on equality, so it is safe
    /// against birthday-bound hash collisions.
    pub fn canonical_key(&self, state: &State) -> StateKey {
        let mut s = state.clone();
        if self.strategy != Strategy::OptimalMove {
            s.normalize(&self.map);
        }
        StateKey::new(s)
    }
}

// =====================================================================
// Search loops.
// =====================================================================

impl Solver {
    /// Searches for a solution using A* / weighted A* depending on the strategy.
    ///
    /// Maintains a best-known `g` score for each state key and **reopens**
    /// states when a cheaper path is found. Reopening is required for
    /// correctness with non-uniform edge costs (move-optimal search) and is
    /// still useful in push-optimal search even though edges are unit-cost.
    pub fn a_star_search(&self) -> Result<Actions, SearchError> {
        let mut heap = BinaryHeap::new();

        // Best known cost-to-come g(key). The key carries the full canonical
        // state, so collisions are impossible.
        let mut best_g: FxHashMap<StateKey, i32> = FxHashMap::default();
        let mut parent: FxHashMap<StateKey, StateKey> = FxHashMap::default();

        let start: State = self.map.clone().into();
        let start_node = Node::new(start, 0, 0, self);
        let start_key = StateKey::new(start_node.state.clone());

        best_g.insert(start_key, 0);
        heap.push(start_node);

        let mut terminator = TerminatorInner::new(self.terminator);

        while let Some(node) = heap.pop() {
            if terminator.tick() {
                return Err(SearchError::Terminated);
            }

            let g_here = self.node_g(&node);
            let node_key = StateKey::new(node.state.clone());

            // Stale: another path to this state was found cheaper after we
            // queued this node. Skip — we'll process the cheaper path's copy.
            if best_g.get(&node_key).copied() != Some(g_here) {
                continue;
            }

            if node.state.is_solved(self) {
                return Ok(reconstruct::actions_from_chain(&self.map, &node_key, &parent));
            }

            for succ in node.successors(self) {
                let g_succ = self.node_g(&succ);
                let succ_key = StateKey::new(succ.state.clone());
                let old = best_g.get(&succ_key).copied().unwrap_or(i32::MAX);

                if g_succ >= old {
                    continue;
                }

                best_g.insert(succ_key.clone(), g_succ);
                parent.insert(succ_key, node_key.clone());
                heap.push(succ);
            }
        }

        Err(SearchError::NoSolution)
    }

    /// Solves the Sokoban level using **IDA\*** (Iterative Deepening A*).
    ///
    /// IDA* performs repeated depth-first searches with increasing `f = g + h`
    /// thresholds. It can be significantly more memory-efficient than A* but
    /// typically expands more nodes.
    ///
    /// # Cost model
    ///
    /// * `OptimalPush` / `Fast`: `g = pushes`
    /// * `OptimalMove`: `g = moves` (usually much slower)
    ///
    /// # Returns
    ///
    /// * `Ok(())` if a solution is found.
    /// * `Err(SearchError::NoSolution)` if no solution exists.
    /// * `Err(SearchError::Terminated)` if the terminator triggers.
    pub fn ida_star_search(&self) -> Result<(), SearchError> {
        let start: State = self.map.clone().into();
        let h0 = start.heuristic(self);
        if h0 == i32::MAX {
            return Err(SearchError::NoSolution);
        }

        let mut threshold = h0; // g(start)=0
        let mut terminator = TerminatorInner::new(self.terminator);

        loop {
            let mut visited: FxHashSet<StateKey> = FxHashSet::default();

            match self.ida_dfs(&start, 0, 0, threshold, &mut visited, &mut terminator) {
                IdaDfsResult::Found => return Ok(()),
                IdaDfsResult::Terminated => return Err(SearchError::Terminated),
                IdaDfsResult::NextThreshold(next) => {
                    if next == i32::MAX {
                        return Err(SearchError::NoSolution);
                    }
                    threshold = next;
                }
            }
        }
    }

    /// Performs one IDA* depth-first search iteration for a given `threshold`.
    ///
    /// Uses **path-based** cycle checking via `visited`: the current state's
    /// key is inserted on entry and removed on return, so only ancestors on
    /// the current DFS path are considered "seen."
    fn ida_dfs(
        &self,
        state: &State,
        pushes: i32,
        moves: i32,
        threshold: i32,
        visited: &mut FxHashSet<StateKey>,
        terminator: &mut TerminatorInner,
    ) -> IdaDfsResult {
        if terminator.tick() {
            return IdaDfsResult::Terminated;
        }

        let h = state.heuristic(self);
        if h == i32::MAX {
            return IdaDfsResult::NextThreshold(i32::MAX);
        }

        let g = match self.strategy {
            Strategy::OptimalMove => moves,
            Strategy::OptimalPush | Strategy::Fast => pushes,
            // Greedy in IDA* is unusual but well-defined: f = h, so
            // thresholds iterate on heuristic values. Convergence is poor
            // — prefer `a_star_search` for greedy.
            Strategy::Greedy => 0,
        };
        let f = g.saturating_add(h);

        if f > threshold {
            return IdaDfsResult::NextThreshold(f);
        }
        if state.is_solved(self) {
            return IdaDfsResult::Found;
        }

        let key = self.canonical_key(state);
        if !visited.insert(key.clone()) {
            // Cycle on the current DFS path; ignore.
            return IdaDfsResult::NextThreshold(i32::MAX);
        }

        let node = Node::new(state.clone(), pushes, moves, self);
        let mut min_next = i32::MAX;

        for succ in node.successors(self) {
            match self.ida_dfs(
                &succ.state,
                succ.pushes,
                succ.moves,
                threshold,
                visited,
                terminator,
            ) {
                IdaDfsResult::Found => {
                    visited.remove(&key);
                    return IdaDfsResult::Found;
                }
                IdaDfsResult::Terminated => {
                    visited.remove(&key);
                    return IdaDfsResult::Terminated;
                }
                IdaDfsResult::NextThreshold(t) => {
                    if t < min_next {
                        min_next = t;
                    }
                }
            }
        }

        visited.remove(&key);
        IdaDfsResult::NextThreshold(min_next)
    }

    /// Returns the cost-to-come `g` for a node based on the active strategy.
    fn node_g(&self, node: &Node) -> i32 {
        match self.strategy {
            Strategy::OptimalMove => node.moves,
            Strategy::OptimalPush | Strategy::Fast => node.pushes,
            // Greedy ignores g; using a constant turns the reopen check into
            // visited-once semantics (g_succ = 0 is never strictly less than
            // the recorded 0, so a state is processed only the first time).
            Strategy::Greedy => 0,
        }
    }
}
