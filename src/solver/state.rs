use std::hash::{DefaultHasher, Hash, Hasher};

use crate::{
    box_set::BoxSet,
    matching::min_cost_matching,
    math::IVector2,
    path_finding::{normalized_area, reachable_area},
    Map, Tiles,
};

use super::Solver;

/// A Sokoban state: where the player and every box currently sit.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct State {
    pub player_position: IVector2,
    pub box_positions: BoxSet,
}

impl State {
    /// Returns true iff every box sits on a goal.
    pub fn is_solved(&self, solver: &Solver) -> bool {
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

    /// Returns an admissible push-count lower bound from the bipartite matching
    /// of boxes to goals over the precomputed empty-board push distances.
    ///
    /// This is much tighter than summing each box's nearest-goal distance —
    /// the matching enforces the one-to-one assignment constraint. If no
    /// perfect matching exists (some box can't reach any goal in the abstraction),
    /// returns `i32::MAX` to mark the state as a provable dead end.
    ///
    /// The result is cached by box configuration on the solver, since the
    /// matching depends only on box positions and many states share the same
    /// `BoxSet` during a search.
    pub fn heuristic(&self, solver: &Solver) -> i32 {
        if let Some(cached) = solver.cached_heuristic(&self.box_positions) {
            return cached;
        }

        let h = self.compute_heuristic(solver);
        solver.cache_heuristic(self.box_positions.clone(), h);
        h
    }

    fn compute_heuristic(&self, solver: &Solver) -> i32 {
        let goals: Vec<IVector2> = solver.map().goal_positions().iter().copied().collect();
        let boxes: Vec<IVector2> = self.box_positions.iter().collect();

        if boxes.len() != goals.len() {
            return i32::MAX;
        }
        if boxes.is_empty() {
            return 0;
        }

        let dm = solver.distance_matrix();
        let n = boxes.len();

        // Flat row-major cost matrix — one allocation instead of `n + 1`.
        let mut cost = vec![0i32; n * n];
        for (i, b) in boxes.iter().enumerate() {
            let row = dm.get(b);
            for (j, g) in goals.iter().enumerate() {
                cost[i * n + j] = row
                    .and_then(|m| m.get(g).copied())
                    .unwrap_or(i32::MAX);
            }
        }

        match min_cost_matching(&cost, n) {
            Some((c, _)) => c,
            None => i32::MAX,
        }
    }

    /// Normalizes the player position to the top-left of its reachable area.
    ///
    /// Two states with identical box configurations and player positions in
    /// the same reachable region collapse to the same normalized state. Used
    /// by push-space search to deduplicate states that differ only in the
    /// player's standing position within an open region.
    pub fn normalize(&mut self, map: &Map) {
        self.player_position = normalized_area(&reachable_area(self.player_position, |position| {
            !(map[position].intersects(Tiles::Wall) || self.box_positions.contains(&position))
        }))
        .unwrap();
    }
}

impl Hash for State {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.player_position.hash(state);
        // BoxSet hashes deterministically (raw bit-array order is fixed).
        self.box_positions.hash(state);
    }
}

impl From<Map> for State {
    fn from(map: Map) -> Self {
        Self {
            player_position: map.player_position(),
            box_positions: map.box_positions().clone(),
        }
    }
}

/// A collision-safe transposition-table key.
///
/// Wraps a fully canonicalized [`State`] together with a precomputed 64-bit
/// hash. The cached hash makes hash-map lookups very cheap — the [`Hash`]
/// implementation forwards to the cached `u64` — while equality compares the
/// *full* state, so the rare two states that happen to share a `u64` digest
/// are still distinguished correctly.
///
/// Why this matters: the search loop performs ~10⁶–10⁹ transposition-table
/// lookups, and a `u64`-only key produces silent collisions at the birthday
/// bound. A collision corrupts `best_g` and the parent chain, giving wrong
/// answers with no warning.
///
/// The contained state must already be canonical for the caller's strategy.
/// Use `Solver::canonical_key` to construct one safely from any state.
#[derive(Clone, Debug)]
pub struct StateKey {
    state: State,
    hash: u64,
}

impl StateKey {
    /// Builds a key from an already-canonicalized state.
    pub fn new(state: State) -> Self {
        let mut hasher = DefaultHasher::new();
        state.hash(&mut hasher);
        Self {
            state,
            hash: hasher.finish(),
        }
    }

    /// Returns the canonical state behind this key.
    pub fn state(&self) -> &State {
        &self.state
    }
}

impl PartialEq for StateKey {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Cheap reject on hash mismatch, full check on hit.
        self.hash == other.hash && self.state == other.state
    }
}

impl Eq for StateKey {}

impl Hash for StateKey {
    #[inline]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.hash.hash(hasher);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn make_state(player: (i32, i32), boxes: &[(i32, i32)]) -> State {
        let mut box_positions = BoxSet::new(8);
        for (x, y) in boxes {
            box_positions.insert(IVector2::new(*x, *y));
        }
        State {
            player_position: IVector2::new(player.0, player.1),
            box_positions,
        }
    }

    #[test]
    fn equal_states_produce_equal_keys() {
        let a = StateKey::new(make_state((1, 1), &[(2, 2), (3, 3)]));
        let b = StateKey::new(make_state((1, 1), &[(3, 3), (2, 2)]));
        assert_eq!(a, b, "BoxSet is order-independent; keys must match");
    }

    #[test]
    fn different_states_produce_different_keys() {
        let a = StateKey::new(make_state((1, 1), &[(2, 2)]));
        let b = StateKey::new(make_state((1, 1), &[(2, 3)]));
        assert_ne!(a, b);
    }

    #[test]
    fn keys_round_trip_through_hashmap() {
        let mut map: HashMap<StateKey, i32> = HashMap::new();
        let key = StateKey::new(make_state((1, 1), &[(2, 2)]));
        map.insert(key.clone(), 42);

        // A freshly-built key for the same state must hit the same entry.
        let lookup = StateKey::new(make_state((1, 1), &[(2, 2)]));
        assert_eq!(map.get(&lookup), Some(&42));
    }
}


