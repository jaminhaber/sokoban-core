//! A solver for the Sokoban problem.

use crate::{
    direction::Direction,
    math::IVector2,
    node::Node,
    path_finding::find_path,
    state::{State, StateKey},
    Action, Actions, Map, SearchError, Tiles,
};
use std::{
    cell::OnceCell,
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
    time::Duration,
};

/// The strategy to use when searching for a solution.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub enum Strategy {
    /// Search quickly for any solution (not necessarily optimal).
    ///
    /// Implemented as **weighted A*** in push space using a strong admissible heuristic.
    #[default]
    Fast,

    /// Find a push-optimal solution (minimum number of pushes).
    OptimalPush,

    /// Find a move-optimal solution (minimum number of player moves).
    OptimalMove,
}

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

    // lower_bounds[pos] = minimum pushes from pos to any goal in the abstraction
    lower_bounds: OnceCell<HashMap<IVector2, i32>>,

    // distance_matrix[pos][goal] = minimum pushes from pos to that goal in the abstraction
    distance_matrix: OnceCell<HashMap<IVector2, HashMap<IVector2, i32>>>,

    // tunnel macros keyed by (box_position, push_direction)
    tunnels: OnceCell<HashSet<(IVector2, Direction)>>,

    terminator: Terminator,
}

/// How to terminate the search.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub enum Terminator {
    /// Never terminate.
    #[default]
    None,
    /// Terminate after a timeout.
    Timeout(Duration),
    /// Terminate after a number of iterations.
    Iterations(u64),
}

impl Terminator {
    /// Creates a new `Terminator` that terminates after the specified number of iterations.
    pub fn new_iterations(max_iterations: u64) -> Self {
        Self::Iterations(max_iterations)
    }

    /// Creates a new `Terminator` that terminates after the specified number of seconds.
    pub fn new_duration_secs(secs: u64) -> Self {
        Self::Timeout(Duration::from_secs(secs))
    }
}

struct TerminatorInner {
    terminator: Terminator,
    iterations: u64,
    start_time: std::time::Instant,
}

impl TerminatorInner {
    fn new(terminator: Terminator) -> Self {
        Self {
            terminator,
            iterations: 0,
            start_time: std::time::Instant::now(),
        }
    }

    fn tick(&mut self) -> bool {
        self.iterations += 1;
        match self.terminator {
            Terminator::None => false,
            Terminator::Timeout(duration) => self.start_time.elapsed() >= duration,
            Terminator::Iterations(max_iterations) => self.iterations >= max_iterations,
        }
    }
}
/// Internal return type for IDA* DFS.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IdaDfsResult {
    /// A goal state was found in the current iteration.
    Found,
    /// No goal found; the next iteration should use this threshold.
    NextThreshold(i32),
    /// The terminator requested early stop.
    Terminated,
}

impl Solver {
    /// Creates a new `Solver`.
    ///
    /// Expensive preprocessing (push distances, dead squares, tunnels) is computed lazily
    /// the first time it is needed.
    pub fn new(map: Map, strategy: Strategy) -> Self {
        Self {
            map,
            strategy,
            fast_weight: 2.0,
            tunnel_macros: false,
            lower_bounds: OnceCell::new(),
            distance_matrix: OnceCell::new(),
            tunnels: OnceCell::new(),
            terminator: Terminator::None,
        }
    }

    /// Enables or disables tunnel macro compression.
    ///
    /// When enabled, successor generation will repeatedly push boxes through
    /// detected tunnel corridors as a single successor, reducing search depth.
    ///
    /// This is a performance optimization. Keep it disabled if you suspect your
    /// tunnel detector is overly aggressive for some level sets.
    pub fn with_tunnel_macros(mut self, enabled: bool) -> Self {
        self.tunnel_macros = enabled;
        self
    }

    /// Returns whether tunnel macro compression is enabled.
    pub fn tunnel_macros(&self) -> bool {
        self.tunnel_macros
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

    /// Returns a transposition-table key for `state` under this solver's strategy.
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
        let mut best_g: HashMap<StateKey, i32> = HashMap::new();
        let mut parent: HashMap<StateKey, StateKey> = HashMap::new();

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
                return Ok(self.construct_actions(&node_key, &parent));
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

    /// Returns the cost-to-come `g` for a node based on the active strategy.
    fn node_g(&self, node: &Node) -> i32 {
        match self.strategy {
            Strategy::OptimalMove => node.moves,
            Strategy::OptimalPush | Strategy::Fast => node.pushes,
        }
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
            let mut visited: HashSet<StateKey> = HashSet::new();

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
        visited: &mut HashSet<StateKey>,
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

    /// Returns lower bounds (dead-square / min-to-any-goal) computed from push distances.
    pub fn lower_bounds(&self) -> &HashMap<IVector2, i32> {
        self.lower_bounds
            .get_or_init(|| self.precompute_push_distances().0)
    }

    /// Returns the push distance matrix used by the matching heuristic.
    pub fn distance_matrix(&self) -> &HashMap<IVector2, HashMap<IVector2, i32>> {
        self.distance_matrix
            .get_or_init(|| self.precompute_push_distances().1)
    }

    /// Returns tunnel macro table.
    pub fn tunnels(&self) -> &HashSet<(IVector2, Direction)> {
        self.tunnels.get_or_init(|| self.calculate_tunnels())
    }

    /// Returns whether `(box_pos, dir)` is a tunnel macro step.
    pub fn is_tunnel(&self, box_pos: IVector2, dir: Direction) -> bool {
        self.tunnels().contains(&(box_pos, dir))
    }

    /// Precomputes push distances in an admissible abstraction.
    ///
    /// Builds the *push graph* for a box on an empty board:
    /// a box can move from `prev` to `cur = prev + dir` iff:
    /// - `prev` is not a wall
    /// - `cur` is not a wall
    /// - `prev - dir` is not a wall (player can stand behind)
    ///
    /// Distances are computed by reverse BFS from each goal.
    fn precompute_push_distances(
        &self,
    ) -> (
        HashMap<IVector2, i32>,
        HashMap<IVector2, HashMap<IVector2, i32>>,
    ) {
        let is_free =
            |p: IVector2| -> bool { self.map.in_bounds(p) && !self.map[p].intersects(Tiles::Wall) };

        let mut distance_matrix: HashMap<IVector2, HashMap<IVector2, i32>> = HashMap::new();

        for &goal in self.map.goal_positions().iter() {
            if !is_free(goal) {
                continue;
            }

            let mut dist_to_goal: HashMap<IVector2, i32> = HashMap::new();
            let mut q = VecDeque::new();

            dist_to_goal.insert(goal, 0);
            q.push_back(goal);

            while let Some(cur) = q.pop_front() {
                let dcur = dist_to_goal[&cur];

                // reverse edges: predecessor prev such that prev + dir = cur
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
                distance_matrix
                    .entry(pos)
                    .or_insert_with(HashMap::new)
                    .insert(goal, d);
            }
        }

        // lower_bounds[pos] = min distance to any goal
        let mut lower_bounds: HashMap<IVector2, i32> = HashMap::new();
        for (pos, gm) in &distance_matrix {
            if let Some(best) = gm.values().min().copied() {
                lower_bounds.insert(*pos, best);
            }
        }

        (lower_bounds, distance_matrix)
    }

    /// Detects static tunnel corridors for macro pushes.
    ///
    /// This is a conservative detector keyed by `(box_pos, dir)`:
    /// if a box at `box_pos` is pushed in `dir` and the corridor walls force
    /// the continuation, we mark that step as a tunnel macro.
    fn calculate_tunnels(&self) -> HashSet<(IVector2, Direction)> {
        let mut tunnels = HashSet::new();

        let is_free =
            |p: IVector2| -> bool { self.map.in_bounds(p) && !self.map[p].intersects(Tiles::Wall) };

        for x in 0..self.map.dimensions().x {
            for y in 0..self.map.dimensions().y {
                let p = IVector2::new(x, y);
                if !is_free(p) || self.map[p].intersects(Tiles::Goal) {
                    continue;
                }

                for dir in Direction::iter() {
                    let forward = p + &dir.into();
                    if !is_free(forward) || self.map[forward].intersects(Tiles::Goal) {
                        continue;
                    }

                    let (l, r) = dir.perpendiculars();
                    let lp = p + &l.into();
                    let rp = p + &r.into();
                    if !self.map.in_bounds(lp)
                        || !self.map.in_bounds(rp)
                        || !self.map[lp].intersects(Tiles::Wall)
                        || !self.map[rp].intersects(Tiles::Wall)
                    {
                        continue;
                    }

                    // also require the forward cell is corridor-like
                    let flp = forward + &l.into();
                    let frp = forward + &r.into();
                    if !self.map.in_bounds(flp)
                        || !self.map.in_bounds(frp)
                        || !self.map[flp].intersects(Tiles::Wall)
                        || !self.map[frp].intersects(Tiles::Wall)
                    {
                        continue;
                    }

                    // Only mark as tunnel if the abstraction says forward isn't a dead square
                    // (avoid macro into unreachable corridors).
                    if !self.lower_bounds().contains_key(&forward) {
                        continue;
                    }

                    tunnels.insert((forward, dir));
                }
            }
        }

        tunnels
    }

    /// Reconstructs the player's action sequence from the solved-state key chain.
    ///
    /// The search runs in *push space* with player positions canonicalized, so
    /// the player coordinate stored on each canonical state is not necessarily
    /// where the player would actually be at that point in the playthrough.
    /// To produce valid `Move`/`Push` actions we therefore:
    ///
    /// 1. Walk the parent chain to recover the sequence of pushes intended.
    /// 2. Simulate forward from the real initial state, finding a player path
    ///    to the cell behind each push using the *current simulated* state.
    fn construct_actions(
        &self,
        goal_key: &StateKey,
        parent: &HashMap<StateKey, StateKey>,
    ) -> Actions {
        // 1) Build the chain of canonical states from start → goal.
        let mut chain: Vec<&StateKey> = vec![goal_key];
        let mut k = goal_key;
        while let Some(pk) = parent.get(k) {
            chain.push(pk);
            k = pk;
        }
        chain.reverse();

        // 2) Convert each state-to-state transition into a push step.
        #[derive(Clone, Copy, Debug)]
        struct PushStep {
            from: IVector2,
            dir: Direction,
            count: i32,
        }

        let mut steps: Vec<PushStep> = Vec::with_capacity(chain.len().saturating_sub(1));
        for win in chain.windows(2) {
            let prev = win[0].state();
            let cur = win[1].state();

            let prev_box = prev
                .box_positions
                .difference(&cur.box_positions)
                .next()
                .expect("no removed box");
            let cur_box = cur
                .box_positions
                .difference(&prev.box_positions)
                .next()
                .expect("no added box");

            let diff = cur_box - prev_box;
            let dir = Direction::try_from(IVector2::new(diff.x.signum(), diff.y.signum()))
                .expect("non-axis-aligned displacement");
            let count = diff.x.abs() + diff.y.abs();
            debug_assert!(count >= 1);

            steps.push(PushStep {
                from: prev_box,
                dir,
                count,
            });
        }

        // 3) Simulate forward from the actual initial state, emitting moves.
        let mut actions = Actions::new();
        let mut sim_state: State = self.map.clone().into();

        for step in steps {
            let mut box_pos = step.from;
            for _ in 0..step.count {
                let behind = box_pos - &step.dir.into();

                let path = find_path(sim_state.player_position, behind, |p| {
                    !self.map[p].intersects(Tiles::Wall) && !sim_state.box_positions.contains(&p)
                })
                .expect("no path to behind-square during reconstruction");

                for w in path.windows(2) {
                    let d = Direction::try_from(w[1] - w[0]).unwrap();
                    actions.push(Action::Move(d));
                }

                actions.push(Action::Push(step.dir));

                let new_box = box_pos + &step.dir.into();
                debug_assert!(!self.map[new_box].intersects(Tiles::Wall));
                debug_assert!(!sim_state.box_positions.contains(&new_box));
                debug_assert!(sim_state.box_positions.contains(&box_pos));

                sim_state.box_positions.remove(box_pos);
                sim_state.box_positions.insert(new_box);
                sim_state.player_position = box_pos;
                box_pos = new_box;
            }
        }

        actions
    }
}
