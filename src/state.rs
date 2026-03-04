use std::hash::{DefaultHasher, Hash, Hasher};

use crate::{
    box_set::BoxSet,
    matching::min_cost_matching,
    math::IVector2,
    path_finding::{normalized_area, reachable_area},
    solver::Solver,
    Map, Tiles,
};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct State {
    pub player_position: IVector2,
    pub box_positions: BoxSet,
}

impl State {
    /// Returns true if the state is solved.
    pub fn is_solved(&self, solver: &Solver) -> bool {
        // Check if all box positions match goal positions
        if self.box_positions.len() != solver.map().goal_positions().len() {
            return false;
        }
        for box_pos in &self.box_positions {
            if !solver.map().goal_positions().contains(&box_pos) {
                return false;
            }
        }
        true
    }

    /// Returns an admissible heuristic estimate for the state.
///
/// This heuristic is the **minimum-cost perfect matching** between boxes and goals.
/// The cost of assigning a given box to a given goal is the precomputed *push distance*
/// from the box position to the goal, ignoring other boxes.
///
/// This is a much tighter lower bound than summing each box's nearest-goal distance,
/// because it respects the one-to-one assignment constraint.
///
/// If no perfect matching exists (some box cannot reach any goal in the abstraction),
/// this function returns `i32::MAX` to signal a provable dead end in the abstraction.
pub fn heuristic(&self, solver: &Solver) -> i32 {
        let goals: Vec<IVector2> = solver.map().goal_positions().iter().copied().collect();
        let boxes: Vec<IVector2> = self.box_positions.iter().collect();

        if boxes.len() != goals.len() {
            return i32::MAX;
        }
        if boxes.is_empty() {
            return 0;
        }

        let dm = solver.distance_matrix();

        let cost_matrix: Vec<Vec<i32>> = boxes
            .iter()
            .map(|b| {
                goals
                    .iter()
                    .map(|g| dm.get(b).and_then(|m| m.get(g).copied()).unwrap_or(i32::MAX))
                    .collect()
            })
            .collect();

        match min_cost_matching(&cost_matrix) {
            Some((c, _)) => c,
            None => i32::MAX,
        }
    }

    /// Normalizes the state.
    pub fn normalize(&mut self, map: &Map) {
        self.player_position = normalized_area(&reachable_area(self.player_position, |position| {
            !(map[position].intersects(Tiles::Wall) || self.box_positions.contains(&position))
        }))
        .unwrap();
    }

    /// Returns the hash key for push-optimal search.
    /// Normalizes player position to the top-left of the reachable area.
    /// This is safe for push-optimal because player position within a reachable
    /// region doesn't affect push count.
    pub fn key_push(&self, map: &Map) -> u64 {
        let mut normalized_state = self.clone();
        normalized_state.normalize(map);
        let mut hasher = DefaultHasher::new();
        normalized_state.hash(&mut hasher);
        hasher.finish()
    }

    /// Returns the hash key for move-optimal search.
    /// Uses exact player position because different player positions with
    /// the same box configuration can have different future move costs.
    pub fn key_move(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl Hash for State {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.player_position.hash(state);
        // BoxSet already hashes deterministically (bits are in fixed order)
        self.box_positions.hash(state);
    }
}/// Returns the hash key for push-optimal search assuming the state is already normalized.
///
/// This avoids re-running reachability normalization when the caller has already
/// canonicalized `player_position` with `normalize()`.
pub fn key_push_canonical(&self) -> u64 {
    let mut hasher = DefaultHasher::new();
    self.hash(&mut hasher);
    hasher.finish()
}


