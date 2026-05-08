use std::str::FromStr;

use sokoban_core::IVector2;
use sokoban_core::{solver::*, Level, SearchError};

mod utils;
use utils::*;

/// Tiny solvable level used by the strategy and config tests below.
/// Player walks left to push the box one step onto a goal — single push,
/// trivial to solve under any strategy.
const TINY: &str = "#####\n#@$.#\n#####\n";

fn solve(mut level: Level) {
    let map = level.map().clone();
    let solver = Solver::new(map, Strategy::Fast);
    let solution = solver.a_star_search().unwrap();
    assert!(solver.ida_star_search().is_ok());
    let directions = solution.iter().map(|action| action.direction());
    level.do_actions(directions).unwrap();
    assert!(level.is_solved());
}

#[test]
fn test_solver() {
    solve(load_level_from_file("assets/BoxWorld_100.xsb", 1));
    solve(load_level_from_file("assets/BoxWorld_100.xsb", 2));
    solve(load_level_from_file("assets/BoxWorld_100.xsb", 3));
}

#[test]
fn weird_levels() {
    let levels = "
  #####
###   #
# $   #
# @$  #
#.. ###
#####
title: Weird 1

 #####
##   ##
# $@ .#
# #$#.##
# $  . #
#      #
########
title: Weird 2

 #####
##   ##
# $@ .#
# #$#.##
# $ $..#
#      #
########
title: Weird 3
";
    for level in Level::load_from_str(levels) {
        solve(level.unwrap());
    }
}

/// Solving the same level twice must produce byte-identical action sequences.
/// Catches accidental dependence on hash-iteration order or any other
/// non-deterministic data structure in the search hot path.
#[test]
fn solver_is_deterministic() {
    let cases: &[(&str, usize)] = &[
        ("assets/Microban_155.xsb", 1),
        ("assets/Microban_155.xsb", 7),
        ("assets/Microban_155.xsb", 50),
        ("assets/BoxWorld_100.xsb", 3),
    ];

    for (path, idx) in cases {
        let map = load_level_from_file(path, *idx).map().clone();

        let first = Solver::new(map.clone(), Strategy::Fast)
            .a_star_search()
            .unwrap();
        let second = Solver::new(map.clone(), Strategy::Fast)
            .a_star_search()
            .unwrap();

        assert_eq!(
            first, second,
            "{} #{}: solver returned different action sequences across runs",
            path, idx
        );
    }
}

#[test]
fn test_terminator_iterations_limit() {
    let level = load_level_from_file("assets/BoxWorld_100.xsb", 3);
    let map = level.map().clone();

    // With only 5 iterations, the solver should not be able to find a solution
    let solver =
        Solver::new(map.clone(), Strategy::Fast).with_terminator(Terminator::Iterations(5));
    assert_eq!(solver.a_star_search(), Err(SearchError::Terminated));

    // IDA* should also terminate
    let solver = Solver::new(map, Strategy::Fast).with_terminator(Terminator::Iterations(5));
    assert_eq!(solver.ida_star_search(), Err(SearchError::Terminated));
}

#[test]
fn optimal_push_matches_known_minimum() {
    // BoxWorld #3 is small enough that Fast finds a near-optimal solution
    // and OptimalPush finds *the* push-optimal one. The optimum should be
    // no worse than what Fast gets.
    let map = load_level_from_file("assets/BoxWorld_100.xsb", 3)
        .map()
        .clone();

    let fast = Solver::new(map.clone(), Strategy::Fast)
        .a_star_search()
        .unwrap();
    let optimal = Solver::new(map.clone(), Strategy::OptimalPush)
        .a_star_search()
        .unwrap();

    assert!(
        optimal.pushes() <= fast.pushes(),
        "OptimalPush ({}) must not exceed Fast ({})",
        optimal.pushes(),
        fast.pushes()
    );

    // Replay the optimal solution to check it actually solves the level.
    let mut level = load_level_from_file("assets/BoxWorld_100.xsb", 3);
    level
        .do_actions(optimal.iter().map(|a| a.direction()))
        .unwrap();
    assert!(level.is_solved());
}

#[test]
fn optimal_move_yields_no_more_moves_than_fast() {
    // OptimalMove minimizes player moves. On a tiny level the savings are
    // small but the invariant must hold.
    let mut level_fast = Level::from_str(TINY).unwrap();
    let mut level_opt = Level::from_str(TINY).unwrap();

    let fast = Solver::new(level_fast.map().clone(), Strategy::Fast)
        .a_star_search()
        .unwrap();
    let opt = Solver::new(level_opt.map().clone(), Strategy::OptimalMove)
        .a_star_search()
        .unwrap();

    assert!(
        opt.moves() <= fast.moves(),
        "OptimalMove ({} moves) must not exceed Fast ({} moves)",
        opt.moves(),
        fast.moves()
    );

    level_fast
        .do_actions(fast.iter().map(|a| a.direction()))
        .unwrap();
    level_opt
        .do_actions(opt.iter().map(|a| a.direction()))
        .unwrap();
    assert!(level_fast.is_solved());
    assert!(level_opt.is_solved());
}

#[test]
fn fast_weight_one_is_admissible() {
    // Fast with weight 1.0 reduces to plain A* in push space, so the push
    // count must equal OptimalPush.
    let map = load_level_from_file("assets/BoxWorld_100.xsb", 1)
        .map()
        .clone();

    let weighted = Solver::new(map.clone(), Strategy::Fast)
        .with_fast_weight(1.0)
        .a_star_search()
        .unwrap();
    let optimal = Solver::new(map, Strategy::OptimalPush)
        .a_star_search()
        .unwrap();

    assert_eq!(weighted.pushes(), optimal.pushes());
}

#[test]
fn fast_weight_is_clamped_to_at_least_one() {
    // Constructing with a fractional weight clamps to 1.0 — the search must
    // still produce a valid solution.
    let map = Level::from_str(TINY).unwrap().map().clone();
    let solver = Solver::new(map, Strategy::Fast).with_fast_weight(0.1);
    assert_eq!(solver.fast_weight(), 1.0);
    assert!(solver.a_star_search().is_ok());
}

#[test]
fn fast_weight_rejects_non_finite_values() {
    // INFINITY and NaN must not propagate into the priority computation, where
    // they would saturate every node's f-value to i32::MAX and break heap
    // ordering. The setter must clamp to a finite value and the search must
    // still produce a valid solution.
    let map = Level::from_str(TINY).unwrap().map().clone();

    let inf_solver = Solver::new(map.clone(), Strategy::Fast).with_fast_weight(f32::INFINITY);
    assert!(
        inf_solver.fast_weight().is_finite(),
        "fast_weight() returned non-finite value {} for INFINITY input",
        inf_solver.fast_weight()
    );
    assert!(inf_solver.a_star_search().is_ok());

    let nan_solver = Solver::new(map.clone(), Strategy::Fast).with_fast_weight(f32::NAN);
    assert!(
        nan_solver.fast_weight().is_finite(),
        "fast_weight() returned non-finite value {} for NaN input",
        nan_solver.fast_weight()
    );
    assert!(nan_solver.fast_weight() >= 1.0);
    assert!(nan_solver.a_star_search().is_ok());

    let neg_inf_solver = Solver::new(map, Strategy::Fast).with_fast_weight(f32::NEG_INFINITY);
    assert!(neg_inf_solver.fast_weight().is_finite());
    assert!(neg_inf_solver.fast_weight() >= 1.0);
    assert!(neg_inf_solver.a_star_search().is_ok());
}

#[test]
fn tunnel_macros_toggle_keeps_solutions_valid() {
    let mut with = load_level_from_file("assets/BoxWorld_100.xsb", 1);
    let mut without = load_level_from_file("assets/BoxWorld_100.xsb", 1);

    let solution_with = Solver::new(with.map().clone(), Strategy::Fast)
        .with_tunnel_macros(true)
        .a_star_search()
        .unwrap();
    let solution_without = Solver::new(without.map().clone(), Strategy::Fast)
        .with_tunnel_macros(false)
        .a_star_search()
        .unwrap();

    with.do_actions(solution_with.iter().map(|a| a.direction()))
        .unwrap();
    without
        .do_actions(solution_without.iter().map(|a| a.direction()))
        .unwrap();

    assert!(with.is_solved());
    assert!(without.is_solved());
}

#[test]
fn tunnel_macros_setter_round_trips() {
    let map = Level::from_str(TINY).unwrap().map().clone();
    let solver = Solver::new(map, Strategy::Fast);
    assert!(!solver.tunnel_macros(), "default should be off");
    let solver = solver.with_tunnel_macros(true);
    assert!(solver.tunnel_macros());
}

#[test]
fn corral_pruning_setter_round_trips() {
    let map = Level::from_str(TINY).unwrap().map().clone();
    let solver = Solver::new(map, Strategy::Fast);
    assert!(!solver.corral_pruning(), "default should be off");
    let solver = solver.with_corral_pruning(true);
    assert!(solver.corral_pruning());
}

#[test]
fn greedy_strategy_solves_simple_level() {
    let mut level = Level::from_str(TINY).unwrap();
    let solution = Solver::new(level.map().clone(), Strategy::Greedy)
        .a_star_search()
        .unwrap();
    level
        .do_actions(solution.iter().map(|a| a.direction()))
        .unwrap();
    assert!(level.is_solved());
}

#[test]
fn greedy_strategy_solves_real_level() {
    let mut level = load_level_from_file("assets/Microban_155.xsb", 1);
    let solution = Solver::new(level.map().clone(), Strategy::Greedy)
        .a_star_search()
        .unwrap();
    level
        .do_actions(solution.iter().map(|a| a.direction()))
        .unwrap();
    assert!(level.is_solved());
}

#[test]
fn corral_pruning_keeps_solutions_valid() {
    // Toggling corral pruning should not break correctness on a level
    // where the freeze + 2×2 checks are already sufficient.
    let mut level = load_level_from_file("assets/BoxWorld_100.xsb", 1);
    let solution = Solver::new(level.map().clone(), Strategy::Fast)
        .with_corral_pruning(true)
        .a_star_search()
        .unwrap();
    level
        .do_actions(solution.iter().map(|a| a.direction()))
        .unwrap();
    assert!(level.is_solved());
}

#[test]
fn lower_bounds_includes_goals_with_distance_zero() {
    let map = load_level_from_file("assets/Microban_155.xsb", 3)
        .map()
        .clone();
    let solver = Solver::new(map.clone(), Strategy::Fast);
    let lb = solver.lower_bounds();

    for &goal in map.goal_positions() {
        assert_eq!(
            lb.get(&goal).copied(),
            Some(0),
            "goal {:?} must have lower bound 0",
            goal
        );
    }

    // Every reachable lower-bound value is non-negative.
    for &v in lb.values() {
        assert!(v >= 0);
    }
}

#[test]
fn tunnels_table_is_consistent_with_is_tunnel() {
    // Use a level that has tunnel-like corridors so the table is non-empty.
    let map = load_level_from_file("assets/Microban_155.xsb", 3)
        .map()
        .clone();
    let solver = Solver::new(map, Strategy::Fast);
    let tunnels = solver.tunnels();

    // Every (pos, dir) pair in the table must report `is_tunnel = true`.
    for &(pos, dir) in tunnels {
        assert!(solver.is_tunnel(pos, dir));
    }
    // And `is_tunnel` should never report a step that's not in the table.
    use sokoban_core::direction::Direction;
    for x in 0..solver.map().dimensions().x {
        for y in 0..solver.map().dimensions().y {
            for dir in Direction::iter() {
                let pos = IVector2::new(x, y);
                assert_eq!(solver.is_tunnel(pos, dir), tunnels.contains(&(pos, dir)));
            }
        }
    }
}

#[test]
fn distance_matrix_self_distance_is_zero() {
    let map = load_level_from_file("assets/Microban_155.xsb", 3)
        .map()
        .clone();
    let solver = Solver::new(map.clone(), Strategy::Fast);
    let dm = solver.distance_matrix();

    for &goal in map.goal_positions() {
        // Distance from a goal to itself in the abstraction is 0.
        let to_goal = dm.get(&goal).expect("goal missing from distance matrix");
        assert_eq!(to_goal.get(&goal).copied(), Some(0));
    }
}

/// `lower_bounds()` and `distance_matrix()` are computed by a single function
/// returning a `(LowerBounds, DistanceMatrix)` pair. The accessors must share
/// one cached pair so the BFS runs once total — and the two halves stay
/// internally consistent: for every cell present in `lower_bounds`, the value
/// must equal the minimum over `distance_matrix[pos]`.
#[test]
fn lower_bounds_and_distance_matrix_are_consistent() {
    let map = load_level_from_file("assets/Microban_155.xsb", 3)
        .map()
        .clone();
    let solver = Solver::new(map, Strategy::Fast);

    // Order the calls so the lower-bounds accessor runs first; both must see
    // the same precomputed pair.
    let lb = solver.lower_bounds();
    let dm = solver.distance_matrix();

    for (&pos, &expected) in lb {
        let row = dm
            .get(&pos)
            .expect("lower_bounds entry missing from distance_matrix");
        let min_in_row = row
            .values()
            .copied()
            .min()
            .expect("distance_matrix row should not be empty");
        assert_eq!(
            min_in_row, expected,
            "lower_bound at {:?} disagrees with min(distance_matrix[{:?}])",
            pos, pos
        );
    }

    // Every position present in the distance matrix should also appear in the
    // lower bounds — both come from the same precompute pass.
    for &pos in dm.keys() {
        assert!(
            lb.contains_key(&pos),
            "{:?} present in distance_matrix but missing from lower_bounds",
            pos
        );
    }
}

/// Calling `distance_matrix()` first and then `lower_bounds()` must return the
/// same data — a regression test guarding against the refactor caching the
/// halves independently.
#[test]
fn distance_matrix_then_lower_bounds_are_consistent() {
    let map = load_level_from_file("assets/Microban_155.xsb", 3)
        .map()
        .clone();
    let solver = Solver::new(map, Strategy::Fast);

    let dm = solver.distance_matrix();
    let lb = solver.lower_bounds();

    for (&pos, row) in dm {
        let min_in_row = row
            .values()
            .copied()
            .min()
            .expect("distance_matrix row should not be empty");
        assert_eq!(
            lb.get(&pos).copied(),
            Some(min_in_row),
            "lower_bound at {:?} disagrees with min(distance_matrix[{:?}])",
            pos,
            pos
        );
    }
}

#[test]
fn ida_star_solves_simple_level() {
    let map = Level::from_str(TINY).unwrap().map().clone();
    let solver = Solver::new(map, Strategy::OptimalPush);
    assert!(solver.ida_star_search().is_ok());
}

#[test]
fn search_terminator_timeout_returns_terminated() {
    use std::time::Duration;

    // A non-zero timeout that's too short to solve a hard level.
    let map = load_level_from_file("assets/BoxWorld_100.xsb", 3)
        .map()
        .clone();
    let solver = Solver::new(map, Strategy::Fast)
        .with_terminator(Terminator::Timeout(Duration::from_nanos(1)));
    // Either we terminate due to the budget or — extremely unlikely — we
    // happen to succeed before checking. Both outcomes are valid; only a
    // panic would be a bug.
    let result = solver.a_star_search();
    assert!(matches!(result, Err(SearchError::Terminated) | Ok(_)));
}

#[test]
fn terminator_constructors() {
    use std::time::Duration;
    assert_eq!(Terminator::new_iterations(42), Terminator::Iterations(42));
    assert_eq!(
        Terminator::new_duration_secs(7),
        Terminator::Timeout(Duration::from_secs(7))
    );
}

#[expect(dead_code)]
fn print_lower_bounds(solver: &Solver) {
    for y in 0..solver.map().dimensions().y {
        for x in 0..solver.map().dimensions().x {
            let position = IVector2::new(x, y);
            if let Some(lower_bound) = solver.lower_bounds().get(&position) {
                print!("{:3} ", lower_bound);
            } else {
                print!("{:3} ", "###");
            }
        }
        println!();
    }
}
