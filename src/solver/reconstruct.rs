//! Action-sequence reconstruction from a solved-state parent chain.
//!
//! Push-space search canonicalizes the player position within its reachable
//! region, so the player coordinate stored on a canonical state is not the
//! actual player position that would result from playing the moves so far.
//! To produce a valid `Move`/`Push` sequence we walk the parent chain to
//! recover the *push intentions*, then simulate forward from the real
//! initial state, finding a player path before each push using the current
//! simulated state.

use rustc_hash::FxHashMap;

use crate::{
    direction::Direction, map::Map, math::IVector2, path_finding::find_path, Action, Actions, Tiles,
};

use super::state::{State, StateKey};

/// Single push intention extracted from two consecutive canonical states.
#[derive(Clone, Copy, Debug)]
struct PushStep {
    from: IVector2,
    dir: Direction,
    count: i32,
}

/// Walks the parent chain from `goal_key` back to the start, then simulates
/// forward to emit a valid `Move`/`Push` action sequence.
pub(super) fn actions_from_chain(
    map: &Map,
    goal_key: &StateKey,
    parent: &FxHashMap<StateKey, StateKey>,
) -> Actions {
    // 1) Recover the chain of canonical states from start → goal.
    let mut chain: Vec<&StateKey> = vec![goal_key];
    let mut k = goal_key;
    while let Some(pk) = parent.get(k) {
        chain.push(pk);
        k = pk;
    }
    chain.reverse();

    // 2) Convert each consecutive state pair into a push step.
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

    // 3) Replay forward from the actual initial state, emitting moves.
    let mut actions = Actions::new();
    let mut sim_state: State = map.clone().into();

    for step in steps {
        let mut box_pos = step.from;
        for _ in 0..step.count {
            let behind = box_pos - &step.dir.into();

            let path = find_path(sim_state.player_position, behind, |p| {
                !map[p].intersects(Tiles::Wall) && !sim_state.box_positions.contains(&p)
            })
            .expect("no path to behind-square during reconstruction");

            for w in path.windows(2) {
                let d = Direction::try_from(w[1] - w[0]).unwrap();
                actions.push(Action::Move(d));
            }

            actions.push(Action::Push(step.dir));

            let new_box = box_pos + &step.dir.into();
            debug_assert!(!map[new_box].intersects(Tiles::Wall));
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
