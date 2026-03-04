use std::hash::{DefaultHasher, Hash, Hasher};

use crate::{
    box_set::BoxSet,
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

    /// Returns the heuristic value of the state.
    pub fn heuristic(&self, solver: &Solver) -> i32 {
        self.box_positions
            .iter()
            .map(|box_position| solver.lower_bounds()[&box_position])
            .sum()
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
}
