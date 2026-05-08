//! A solver for the Sokoban problem.
//!
//! # Quick start
//!
//! ```no_run
//! use sokoban_core::prelude::*;
//! use std::str::FromStr;
//!
//! let map = Map::from_str("#####\n#@$.#\n#####\n").unwrap();
//! let solver = Solver::new(map, Strategy::Fast);
//! let actions = solver.a_star_search().unwrap();
//! assert!(!actions.is_empty());
//! ```
//!
//! # Two orthogonal axes
//!
//! The solver has two independent dials:
//!
//! - **Algorithm** — [`Solver::a_star_search`] vs [`Solver::ida_star_search`].
//!   Trades memory for time.
//! - **Strategy** — [`Strategy::Fast`], [`Strategy::Greedy`],
//!   [`Strategy::OptimalPush`], [`Strategy::OptimalMove`]. Controls the cost
//!   function and which solution dimension is being optimized.
//!
//! ## Choosing an algorithm
//!
//! | | A* | IDA* |
//! |---|---|---|
//! | Memory | grows with reachable state space (potentially GB) | O(solution depth) |
//! | Time | typically faster — never re-expands a state | re-expands states across iterations |
//! | Returns | the [`Actions`] sequence | only `Ok(())` |
//! | Use when | enough RAM, want the actual moves | A* is OOMing, or you only need solvability |
//!
//! ## Choosing a strategy
//!
//! | Strategy | `g` | priority `f` | Optimality |
//! |---|---|---|---|
//! | [`Strategy::Fast`] *(default)* | pushes | `g + w·h`, `w = 2` | within `w×` factor of optimum |
//! | [`Strategy::Greedy`] | 0 | `h` only | not optimal — can be much longer |
//! | [`Strategy::OptimalPush`] | pushes | `g + h` | minimum push count |
//! | [`Strategy::OptimalMove`] | moves | `g + h` | minimum player-step count |
//!
//! All four use the same heuristic `h`: minimum-cost bipartite matching of
//! boxes to goals over the empty-board push-distance abstraction. It is
//! admissible (never overestimates), which is what gives the optimal modes
//! their guarantee and `Fast`'s bounded-suboptimal guarantee.
//!
//! `Greedy` ignores `g` entirely. It still works inside the [`a_star_search`]
//! framework because setting every node's `g` to zero turns the reopen check
//! into visited-once semantics. IDA* with `Greedy` is well-defined but
//! converges poorly — prefer [`a_star_search`] for greedy.
//!
//! ## Configuration
//!
//! Builder methods on [`Solver`] tune the search:
//!
//! - [`Solver::with_fast_weight`] — override `Fast`'s weight; `w = 1.0`
//!   collapses `Fast` to `OptimalPush`.
//! - [`Solver::with_tunnel_macros`] — compress forced corridor pushes into a
//!   single successor. Cuts depth on tunnel-heavy maps.
//! - [`Solver::with_corral_pruning`] — extra conservative PI-corral deadlock
//!   check on every push. Off by default; expensive on easy levels but useful
//!   on hard ones.
//! - [`Solver::with_terminator`] — bound the search by wall-clock or iteration
//!   count.
//!
//! # Module layout
//!
//! - This file: the [`Solver`] struct, builder methods, and the A* / IDA*
//!   search loops that read directly from the solver's private state.
//! - `strategy`: the [`Strategy`] and [`Terminator`] enums plus the internal
//!   `TerminatorInner` and `IdaDfsResult` types.
//! - `preprocess`: pure functions that compute the push-distance abstraction
//!   (lower bounds + distance matrix) and the tunnel-macro table.
//! - `reconstruct`: walks the parent chain of canonical states and emits a
//!   valid player action sequence, simulating forward from the real initial
//!   state to find walking paths between pushes.
//! - `state` / `node`: the canonical `State` / `StateKey` types and the search
//!   `Node` with its successor generator. Private to the solver module — no
//!   other code in the crate touches them.
//!
//! [`a_star_search`]: Solver::a_star_search

mod node;
mod preprocess;
mod reconstruct;
mod state;
mod strategy;

pub use strategy::{Strategy, Terminator};

use node::Node;
use state::{State, StateKey};
use strategy::{IdaDfsResult, TerminatorInner};

use crate::{box_set::BoxSet, direction::Direction, math::IVector2, Actions, Map, SearchError};
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
/// Construct with [`Solver::new`], optionally chain builder methods, then
/// call [`Solver::a_star_search`] (returns the action sequence) or
/// [`Solver::ida_star_search`] (just confirms solvability with much less
/// memory).
///
/// The solver operates primarily in **push space** — each search edge is a
/// box push, and the player's walking moves between pushes are reconstructed
/// at the end. For [`Strategy::OptimalMove`], push edges are weighted by the
/// player's walking distance to the behind-square plus 1 for the push.
///
/// Expensive preprocessing (push distances, dead squares, tunnel macros) is
/// computed lazily on first access and cached on the solver. Cloning a
/// `Solver` is cheap before any preprocessing runs and copies the caches
/// after. Construct a fresh `Solver` per level if you don't need to share.
///
/// See the [module-level docs](crate::solver) for a comparison of strategies
/// and search algorithms.
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
    /// Larger values tend to find solutions faster but may sacrifice
    /// optimality. Values below 1.0 are clamped to 1.0.
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
    /// Returns lower bounds (dead-square / min-to-any-goal) computed from push
    /// distances.
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
    /// The returned key caches a 64-bit hash for fast hash-map lookups
    /// while comparing the full state on equality, so it is safe against
    /// birthday-bound hash collisions.
    fn canonical_key(&self, state: &State) -> StateKey {
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
    /// Searches for a solution using A* / weighted A* depending on the
    /// strategy. Returns the player's [`Actions`] sequence on success.
    ///
    /// # How it works
    ///
    /// This is the standard A* main loop:
    ///
    /// 1. A `BinaryHeap` open-list keyed on `f = g + w·h`, where `g` is the
    ///    cost-to-come (pushes / moves / 0 depending on the [`Strategy`]) and
    ///    `h` is the bipartite-matching heuristic.
    /// 2. A `best_g` transposition table keyed on a collision-safe canonical
    ///    state key, so already-better-explored states are skipped.
    /// 3. **Reopening** — when a cheaper path to a known state is found, the
    ///    state is requeued. Reopening is required for correctness with
    ///    non-uniform edge costs (move-optimal search) and is still useful in
    ///    push-optimal search even though edges are unit-cost.
    /// 4. On goal pop, the parent chain is walked back to the start and
    ///    forward-simulated with [`crate::path_finding::find_path`] to emit a
    ///    valid `Move`/`Push` sequence.
    ///
    /// # Memory
    ///
    /// Both `best_g` and `parent` grow with the number of distinct states
    /// expanded — potentially hundreds of MB to several GB on hard levels.
    /// If memory matters more than time, use [`Solver::ida_star_search`].
    ///
    /// # Errors
    ///
    /// - [`SearchError::NoSolution`] — the level is unsolvable (or unsolvable
    ///   under the configured [`Strategy`]).
    /// - [`SearchError::Terminated`] — the configured [`Terminator`]'s
    ///   wall-clock or iteration budget was exhausted.
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
                return Ok(reconstruct::actions_from_chain(
                    &self.map, &node_key, &parent,
                ));
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
    /// Returns only `Ok(())` on success — no action sequence.
    ///
    /// # How it works
    ///
    /// IDA* performs repeated depth-first searches with monotonically
    /// increasing `f = g + h` thresholds:
    ///
    /// 1. Start with `threshold = h(start)`.
    /// 2. DFS from the start state, pruning any node whose `f` exceeds the
    ///    threshold and remembering the smallest `f` that was pruned.
    /// 3. If the DFS hits a goal, return `Found`. Otherwise raise the threshold
    ///    to the smallest pruned `f` and DFS again.
    ///
    /// Cycle checking is **path-based**: a state is added to a `visited` set
    /// on entry and removed on return, so only ancestors on the current DFS
    /// path are considered "seen." There is no global transposition table —
    /// that's the trade that buys IDA*'s low memory cost.
    ///
    /// # Memory vs time
    ///
    /// IDA* uses memory proportional to the solution depth (typically a few
    /// hundred bytes), which is dramatically less than A*. The cost is that
    /// states are re-expanded across iterations — IDA* is usually noticeably
    /// slower wall-clock than [`Solver::a_star_search`] on the same level.
    ///
    /// # Cost model
    ///
    /// - `OptimalPush` / `Fast`: `g = pushes`.
    /// - `OptimalMove`: `g = moves`. Much slower because the search space is
    ///   bigger (player position matters).
    /// - `Greedy`: `g = 0`, so `f = h`. Well-defined but converges poorly under
    ///   iterative deepening — prefer [`Solver::a_star_search`] for greedy.
    ///
    /// # Errors
    ///
    /// - [`SearchError::NoSolution`] — exhausted all reachable thresholds.
    /// - [`SearchError::Terminated`] — the [`Terminator`] budget triggered.
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
