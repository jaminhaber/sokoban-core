//! A solver for the Sokoban problem.

use std::{
    cell::OnceCell,
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
    time::Duration,
};
use crate::{
    direction::Direction,
    math::IVector2,
    node::Node,
    path_finding::find_path,
    state::State,
    Action, Actions, Map, SearchError, Tiles,
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
            Terminator::Timeout(duration) => {
                // Check timeout every 1000 iterations to avoid excessive CPU usage
                self.iterations % 1000 == 0 && self.start_time.elapsed() >= duration
            }
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
            lower_bounds: OnceCell::new(),
            distance_matrix: OnceCell::new(),
            tunnels: OnceCell::new(),
            terminator: Terminator::None,
        }
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

    /// Returns the appropriate state key based on the search strategy.
    ///
    /// - `OptimalMove` uses exact player position.
    /// - `OptimalPush` and `Fast` normalize player position within its reachable region.
    pub fn state_key(&self, state: &State) -> u64 {
        match self.strategy {
            Strategy::OptimalMove => state.key_move(),
            Strategy::OptimalPush | Strategy::Fast => state.key_push(&self.map),
        }
    }

    /// Searches for solution using A* / weighted A* depending on the strategy.
    ///
    /// This implementation maintains a best-known `g` score for each state key, allowing
    /// it to **reopen states** when a cheaper path is found. This is required for
    /// correctness for non-uniform costs (move-optimal search) and is still helpful for
    /// push-optimal search.
    pub fn a_star_search(&self) -> Result<Actions, SearchError> {
        let mut heap = BinaryHeap::new();

        // best known cost-to-come g(key)
        let mut best_g: HashMap<u64, i32> = HashMap::new();

        // Parent pointers and state storage for path reconstruction.
        let mut parent: HashMap<u64, u64> = HashMap::new();
        let mut states: HashMap<u64, State> = HashMap::new();

        let start: State = self.map.clone().into();
        let start_node = Node::new(start.clone(), 0, 0, self);
        best_g.insert(start_node.key, 0);
        states.insert(start_node.key, start);

        heap.push(start_node);

        let mut terminator = TerminatorInner::new(self.terminator);

        while let Some(node) = heap.pop() {
            if terminator.tick() {
                return Err(SearchError::Terminated);
            }

            let g_here = self.node_g(&node);

            // stale check: skip if this node is no longer the best known path to its key
            if best_g.get(&node.key).copied() != Some(g_here) {
                continue;
            }

            if node.state.is_solved(self) {
                return Ok(self.construct_actions_from_keys(node.key, &parent, &states));
            }

            for succ in node.successors(self) {
                let g_succ = self.node_g(&succ);
                let old = best_g.get(&succ.key).copied().unwrap_or(i32::MAX);

                if g_succ >= old {
                    continue;
                }

                best_g.insert(succ.key, g_succ);
                parent.insert(succ.key, node.key);
                states.insert(succ.key, succ.state.clone());
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
        let mut visited: HashSet<u64> = HashSet::new();

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
/// This function uses **path-based** cycle checking via `visited`:
/// it inserts the current state's key on entry and removes it on return.
fn ida_dfs(
    &self,
    state: &State,
    pushes: i32,
    moves: i32,
    threshold: i32,
    visited: &mut HashSet<u64>,
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

    let key = self.state_key(state);
    if !visited.insert(key) {
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
        self.lower_bounds.get_or_init(|| self.precompute_push_distances().0)
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
    ) -> (HashMap<IVector2, i32>, HashMap<IVector2, HashMap<IVector2, i32>>) {
        let is_free = |p: IVector2| -> bool {
            self.map.in_bounds(p) && !self.map[p].intersects(Tiles::Wall)
        };

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

        let is_free = |p: IVector2| -> bool {
            self.map.in_bounds(p) && !self.map[p].intersects(Tiles::Wall)
        };

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

    /// Reconstructs the action sequence from a solved state key.
    fn construct_actions_from_keys(
        &self,
        mut key: u64,
        parent: &HashMap<u64, u64>,
        states: &HashMap<u64, State>,
    ) -> Actions {
        let mut actions = Actions::new();

        while let Some(&pk) = parent.get(&key) {
            let cur = states.get(&key).expect("missing state for key");
            let prev = states.get(&pk).expect("missing state for parent key");

            // Identify moved box
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
            let push_dir =
                Direction::try_from(IVector2::new(diff.x.signum(), diff.y.signum())).unwrap();

            // Walk player to behind-square in previous state
            let mut new_actions: Vec<_> = find_path(
                prev.player_position,
                prev_box - &push_dir.into(),
                |p| !self.map[p].intersects(Tiles::Wall) && !prev.box_positions.contains(&p),
            )
            .unwrap()
            .windows(2)
            .map(|w| Direction::try_from(w[1] - w[0]).unwrap())
            .map(Action::Move)
            .collect();

            new_actions.push(Action::Push(push_dir));

            // Tunnel macro expansion: apply the same macro policy as generation.
            let mut b = prev_box + &push_dir.into();
            while self.is_tunnel(b, push_dir) {
                let next = b + &push_dir.into();
                if !self.map.in_bounds(next) || self.map[next].intersects(Tiles::Wall) {
                    break;
                }
                if cur.box_positions.contains(&next) {
                    break;
                }
                b = next;
                new_actions.push(Action::Push(push_dir));
            }

            actions.splice(0..0, new_actions.iter().copied());
            key = pk;
        }

        actions
    }
}
