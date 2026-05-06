use sokoban_core::IVector2;
use sokoban_core::{solver::*, Level, SearchError};

mod utils;
use utils::*;

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
