//! Search-strategy and termination types for [`super::Solver`].

use std::time::{Duration, Instant};

/// The strategy to use when searching for a solution.
///
/// Strategies are points along the speed/optimality trade-off. They differ
/// in the priority function the search uses to rank states:
///
/// | Strategy        | Priority `f`         | Optimality                            |
/// |-----------------|----------------------|---------------------------------------|
/// | [`Fast`]        | `pushes + w·h`       | within `w` × optimum (default `w = 2`) |
/// | [`Greedy`]      | `h`                  | not optimal — can be much longer      |
/// | [`OptimalPush`] | `pushes + h`         | minimum push count                    |
/// | [`OptimalMove`] | `moves + h`          | minimum player-step count             |
///
/// All four strategies use the same heuristic `h`: minimum-cost bipartite
/// matching of boxes to goals over the empty-board push-distance abstraction.
///
/// [`Fast`]: Strategy::Fast
/// [`Greedy`]: Strategy::Greedy
/// [`OptimalPush`]: Strategy::OptimalPush
/// [`OptimalMove`]: Strategy::OptimalMove
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub enum Strategy {
    /// Weighted A* in push space — the default.
    ///
    /// Each state's priority is `pushes + w · h(state)`, where `w` defaults
    /// to `2.0` and is configurable via [`crate::solver::Solver::with_fast_weight`].
    /// Larger `w` makes the search more goal-directed: it prefers states
    /// whose lower-bound estimate looks small, even at the cost of a longer
    /// path so far.
    ///
    /// `w = 1.0` collapses to [`OptimalPush`]. `w = ∞` would collapse to
    /// [`Greedy`].
    ///
    /// **Bounded sub-optimality**: for any `w ≥ 1`, the returned push count
    /// is no more than `w` times the optimum. In practice on Microban the
    /// average factor is ~1.05–1.15, much better than the worst-case bound,
    /// at a fraction of the search cost of pure A*.
    ///
    /// **When to use**: the default. Picks a near-optimal solution quickly.
    /// Choose this unless you specifically need an optimum or specifically
    /// don't care about quality at all.
    ///
    /// [`OptimalPush`]: Strategy::OptimalPush
    /// [`Greedy`]: Strategy::Greedy
    #[default]
    Fast,

    /// Greedy best-first search — picks states purely by heuristic, ignoring
    /// the path cost to reach them.
    ///
    /// Each state's priority is just `h(state)`. The cost-to-come `g` is
    /// effectively zero, so the open-list always pops the state that *looks*
    /// closest to a goal — no matter how long or circuitous the path that
    /// got there was.
    ///
    /// **Why it's fast**: greedy dives. The moment it finds a state with
    /// `h = 0` it returns. It rarely backs out of a region once it's
    /// committed to one.
    ///
    /// **Why solutions can be terrible**: greedy doesn't notice that it took
    /// 200 detour pushes to reach a low-`h` state. It expands "looks close"
    /// states first, even when other regions are objectively closer to the
    /// start. In Sokoban it's particularly easy to fool — the matching
    /// heuristic underestimates the true cost when boxes are near goals but
    /// blocked by walls, and greedy gets drawn to those configurations and
    /// wastes time before finding the real path.
    ///
    /// Implementation note: with `g = 0` everywhere, the existing reopen
    /// logic in [`crate::solver::Solver::a_star_search`] degrades naturally
    /// to "process each state at most once" — exactly the right semantics
    /// for greedy.
    ///
    /// **When to use**: any solution will do (replay generation, tutorial
    /// hints, "just show me *a* way") and [`Fast`] is still too slow on a
    /// particular level. Avoid when solution quality matters.
    ///
    /// [`Fast`]: Strategy::Fast
    Greedy,

    /// Plain A* in push space — returns the minimum-push solution.
    ///
    /// Each state's priority is `pushes + h(state)`. Because the matching
    /// heuristic is admissible (never overestimates the true push count),
    /// A* with weight 1 provably returns the shortest push sequence.
    ///
    /// **Performance**: substantially slower than [`Fast`] because the
    /// search can't bias toward goal-directed expansion — many states share
    /// the same `f`-value and all of them must be explored. Typical
    /// slowdown on hard levels is 5–10× while only shaving a few pushes
    /// off the answer.
    ///
    /// **When to use**: push count is the figure of merit. Sokoban
    /// competition scoring usually counts pushes; use this when comparing
    /// to a known optimum or publishing solutions.
    ///
    /// [`Fast`]: Strategy::Fast
    OptimalPush,

    /// A* with player-walk edge costs — returns the minimum total-moves
    /// solution.
    ///
    /// A "move" is any single 4-direction step by the player, including the
    /// step that performs a push. Each state's priority is
    /// `moves + h(state)`, where `moves` includes the walking distance to
    /// each push's behind-square.
    ///
    /// **Why it's the slowest by a wide margin**: push-space strategies
    /// canonicalize the player position — two states with the same box
    /// configuration but different player positions inside the same
    /// reachable region collapse to one transposition-table key. That free
    /// deduplication doesn't apply here because walking distance from the
    /// player's exact cell affects every future move's cost. The effective
    /// state space balloons accordingly. Typical slowdown vs [`Fast`] is
    /// 50–100× on the same level.
    ///
    /// **When to use**: total move count is the figure of merit. Some
    /// scoring conventions and replay metrics count moves rather than
    /// pushes — use this strategy in those contexts. Otherwise pick a
    /// push-counting strategy.
    ///
    /// [`Fast`]: Strategy::Fast
    OptimalMove,
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

/// Tracks elapsed iterations / wall-clock during a search and answers
/// "should I stop now?" via [`Self::tick`].
pub(super) struct TerminatorInner {
    terminator: Terminator,
    iterations: u64,
    start_time: Instant,
}

impl TerminatorInner {
    pub(super) fn new(terminator: Terminator) -> Self {
        Self {
            terminator,
            iterations: 0,
            start_time: Instant::now(),
        }
    }

    /// Increments the iteration counter and returns `true` if the budget is exhausted.
    pub(super) fn tick(&mut self) -> bool {
        self.iterations += 1;
        match self.terminator {
            Terminator::None => false,
            Terminator::Timeout(duration) => self.start_time.elapsed() >= duration,
            Terminator::Iterations(max_iterations) => self.iterations >= max_iterations,
        }
    }
}

/// Internal return type for IDA* DFS recursion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum IdaDfsResult {
    /// A goal state was found in the current iteration.
    Found,
    /// No goal found; the next iteration should use this threshold.
    NextThreshold(i32),
    /// The terminator requested early stop.
    Terminated,
}
