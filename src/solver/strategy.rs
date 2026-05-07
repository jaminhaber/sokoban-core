//! Search-strategy and termination types for [`super::Solver`].

use std::time::{Duration, Instant};

/// The strategy to use when searching for a solution.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub enum Strategy {
    /// Search quickly for any solution (not necessarily optimal).
    ///
    /// Implemented as **weighted A*** in push space using a strong admissible heuristic.
    #[default]
    Fast,

    /// Greedy best-first search — picks states purely by heuristic, ignoring
    /// the path cost to reach them. Often dramatically faster than `Fast`,
    /// but solutions can be much longer than optimal. Use when you only
    /// need *some* solution as fast as possible.
    Greedy,

    /// Find a push-optimal solution (minimum number of pushes).
    OptimalPush,

    /// Find a move-optimal solution (minimum number of player moves).
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
